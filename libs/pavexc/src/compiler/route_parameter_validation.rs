use std::fmt::Write;

use anyhow::anyhow;
use guppy::graph::PackageGraph;
use indexmap::IndexSet;
use itertools::Itertools;
use miette::Report;
use petgraph::graph::NodeIndex;
use petgraph::Direction;
use rustdoc_types::{ItemEnum, StructKind};

use crate::compiler::analyses::call_graph::{CallGraphNode, RawCallGraph};
use crate::compiler::analyses::components::{ComponentDb, HydratedComponent};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::user_components::{RouterKey, UserComponentId};
use crate::compiler::computation::{Computation, MatchResultVariant};
use crate::compiler::constructors::Constructor;
use crate::compiler::utils::process_framework_path;
use crate::diagnostic;
use crate::diagnostic::{CompilerDiagnostic, LocationExt, OptionalSourceSpanExt};
use crate::language::{GenericArgument, ResolvedType};
use crate::rustdoc::{CrateCollection, GlobalItemId};
use crate::utils::comma_separated_list;

use super::traits::assert_trait_is_implemented;

/// For each handler, check if route parameters are extracted from the URL of the incoming request.
/// If so, check that the type of the route parameter is a struct with named fields and
/// that each named field maps to a route parameter for the corresponding handler.
#[tracing::instrument(name = "Verify route parameters", skip_all)]
pub(crate) fn verify_route_parameters<'a, I>(
    handler_call_graphs: I,
    computation_db: &ComputationDb,
    component_db: &ComponentDb,
    package_graph: &PackageGraph,
    krate_collection: &CrateCollection,
    diagnostics: &mut Vec<miette::Error>,
) where
    I: Iterator<Item = (&'a RouterKey, &'a RawCallGraph)>,
{
    let ResolvedType::ResolvedPath(structural_deserialize) = process_framework_path(
        "pavex::serialization::StructuralDeserialize",
        package_graph,
        krate_collection,
    ) else {
        unreachable!()
    };

    for (router_key, call_graph) in handler_call_graphs {
        let Some((ok_route_params_node_id, ty_)) = call_graph.node_indices().find_map(|node_id| {
            let node = &call_graph[node_id];
            let CallGraphNode::Compute { component_id, .. } = node else { return None; };
            let hydrated_component = component_db.hydrated_component(*component_id, computation_db);
            let HydratedComponent::Constructor(Constructor(Computation::MatchResult(m))) =
                hydrated_component else { return None; };
            if m.variant != MatchResultVariant::Ok {
                return None;
            }
            let ResolvedType::ResolvedPath(ty_) = &m.output else { return None; };
            if ty_.base_type == vec!["pavex", "extract", "route", "RouteParams"] {
                Some((node_id, ty_.clone()))
            } else {
                None
            }
        }) else { continue; };

        let GenericArgument::TypeParameter(extracted_type) = &ty_.generic_arguments[0] else { unreachable!() };

        let Ok(struct_item) = must_be_a_plain_struct(
            component_db,
            package_graph,
            krate_collection,
            diagnostics,
            call_graph,
            ok_route_params_node_id,
            extracted_type,
        ) else {
            continue;
        };
        let ItemEnum::Struct(struct_inner_item) = &struct_item.inner else {
            unreachable!()
        };

        // We only want to check alignment between struct fields and the route parameters in the
        // template if the struct implements `StructuralDeserialize`, our marker trait that stands
        // for "this struct implements serde::Deserialize using a #[derive(serde::Deserialize)] with
        // no customizations (e.g. renames)".
        if assert_trait_is_implemented(krate_collection, extracted_type, &structural_deserialize)
            .is_err()
        {
            continue;
        }

        let route_parameter_names = router_key
            .path
            .split('/')
            .filter_map(|s| s.strip_prefix(':').or_else(|| s.strip_prefix('*')))
            .collect::<IndexSet<_>>();

        let struct_field_names = {
            let mut struct_field_names = IndexSet::new();
            let ResolvedType::ResolvedPath(extracted_path_type) = &extracted_type else { unreachable!() };
            let StructKind::Plain { fields: field_ids, .. } = &struct_inner_item.kind else {
                unreachable!()
            };
            for field_id in field_ids {
                let field_item = krate_collection.get_type_by_global_type_id(&GlobalItemId {
                    rustdoc_item_id: field_id.clone(),
                    package_id: extracted_path_type.package_id.clone(),
                });
                struct_field_names.insert(field_item.name.clone().unwrap());
            }
            struct_field_names
        };

        let non_existing_route_parameters = struct_field_names
            .into_iter()
            .filter(|f| !route_parameter_names.contains(f.as_str()))
            .collect::<IndexSet<_>>();

        if !non_existing_route_parameters.is_empty() {
            report_non_existing_route_parameters(
                component_db,
                package_graph,
                diagnostics,
                router_key,
                call_graph,
                ok_route_params_node_id,
                route_parameter_names,
                non_existing_route_parameters,
                extracted_type,
            )
        }
    }
}

/// Report an error on each compute node that consumes the `RouteParams` extractor
/// while trying to extract one or more route parameters that are not present in
/// the respective route template.
fn report_non_existing_route_parameters(
    component_db: &ComponentDb,
    package_graph: &PackageGraph,
    diagnostics: &mut Vec<Report>,
    router_key: &RouterKey,
    call_graph: &RawCallGraph,
    ok_route_params_node_id: NodeIndex,
    route_parameter_names: IndexSet<&str>,
    non_existing_route_parameters: IndexSet<String>,
    extracted_type: &ResolvedType,
) {
    assert!(!non_existing_route_parameters.is_empty());
    // Find the compute nodes that consume the `RouteParams` extractor and report
    // an error on each of them.
    let consuming_ids =
        route_params_consumer_ids(component_db, call_graph, ok_route_params_node_id);
    for user_component_id in consuming_ids {
        let raw_identifiers = component_db
            .user_component_db()
            .get_raw_callable_identifiers(user_component_id);
        let callable_type = component_db.user_component_db()[user_component_id].callable_type();
        let location = component_db
            .user_component_db()
            .get_location(user_component_id);
        let source = match location.source_file(package_graph) {
            Ok(s) => s,
            Err(e) => {
                diagnostics.push(e.into());
                continue;
            }
        };
        let source_span = diagnostic::get_f_macro_invocation_span(&source, location);
        if route_parameter_names.is_empty() {
            let error = anyhow!(
                    "`{}` is trying to extract route parameters using `RouteParams<{extracted_type:?}>`.\n\
                    But there are no route parameters in `{}`, the corresponding route template!",
                    raw_identifiers.fully_qualified_path().join("::"),
                    router_key.path,
                );
            let d = CompilerDiagnostic::builder(source, error)
                .optional_label(source_span.labeled(format!(
                    "The {callable_type} asking for `RouteParams<{extracted_type:?}>`"
                )))
                .help(
                    "Stop trying to extract route parameters, or add them to the route template!"
                        .into(),
                )
                .build();
            diagnostics.push(d.into());
        } else {
            let missing_msg = if non_existing_route_parameters.len() == 1 {
                let name = non_existing_route_parameters.first().unwrap();
                format!(
                    "There is no route parameter named `{name}`, but there is a struct field named `{name}` \
                    in `{extracted_type:?}`"
                )
            } else {
                let mut msg = "There are no route parameters named ".to_string();
                comma_separated_list(
                    &mut msg,
                    non_existing_route_parameters.iter(),
                    |p| format!("`{p}`"),
                    "or",
                )
                .unwrap();
                write!(
                    &mut msg,
                    ", but they appear as field names in `{extracted_type:?}`"
                )
                .unwrap();
                msg
            };
            let route_parameters = route_parameter_names
                .iter()
                .map(|p| format!("- `{}`", p))
                .join("\n");
            let error = anyhow!(
                    "`{}` is trying to extract route parameters using `RouteParams<{extracted_type:?}>`.\n\
                    Every struct field in `{extracted_type:?}` must be named after one of the route \
                    parameters that appear in `{}`:\n{route_parameters}\n\n\
                    {missing_msg}. This is going to cause a runtime error!",
                    raw_identifiers.fully_qualified_path().join("::"),
                    router_key.path,
                );
            let d = CompilerDiagnostic::builder(source, error)
                .optional_label(source_span.labeled(format!(
                    "The {callable_type} asking for `RouteParams<{extracted_type:?}>`"
                )))
                .help(
                    "Remove or rename the fields that do not map to a valid route parameter."
                        .into(),
                )
                .build();
            diagnostics.push(d.into());
        }
    }
}

/// Checks that the type of the route parameter is a struct with named fields.
/// If it is, returns the rustdoc item for the type.  
/// If it isn't, reports an error diagnostic on each compute node that consumes the
/// `RouteParams` extractor.
fn must_be_a_plain_struct(
    component_db: &ComponentDb,
    package_graph: &PackageGraph,
    krate_collection: &CrateCollection,
    diagnostics: &mut Vec<Report>,
    call_graph: &RawCallGraph,
    ok_route_params_node_id: NodeIndex,
    extracted_type: &ResolvedType,
) -> Result<rustdoc_types::Item, ()> {
    let error_suffix = match extracted_type {
        ResolvedType::ResolvedPath(t) => {
            let Some(item_id) = t.rustdoc_id.clone() else { unreachable!() };
            let item = krate_collection.get_type_by_global_type_id(&GlobalItemId {
                rustdoc_item_id: item_id,
                package_id: t.package_id.clone(),
            });
            match item.inner {
                ItemEnum::Union(_) => format!("`{t:?}` is an union"),
                ItemEnum::Enum(_) => format!("`{t:?}` is an enum"),
                ItemEnum::Struct(ref s) => match &s.kind {
                    StructKind::Unit => {
                        format!("`{t:?}` is a struct with no fields (a.k.a. unit struct)")
                    }
                    StructKind::Tuple(_) => format!("`{t:?}` is a tuple struct"),
                    StructKind::Plain { .. } => return Ok(item.into_owned()),
                },
                _ => unreachable!(),
            }
        }
        ResolvedType::Reference(r) => format!("`{r:?}` is a reference"),
        ResolvedType::Tuple(t) => format!("`{t:?}` is a tuple"),
        ResolvedType::ScalarPrimitive(s) => format!("`{s:?}` is a primitive"),
        ResolvedType::Slice(s) => format!("`{s:?}` is a slice"),
        ResolvedType::Generic(_) => {
            unreachable!()
        }
    };

    // Find the compute nodes that consume the `RouteParams` extractor and report
    // an error on each of them.
    let consuming_ids =
        route_params_consumer_ids(component_db, call_graph, ok_route_params_node_id);

    for user_component_id in consuming_ids {
        let raw_identifiers = component_db
            .user_component_db()
            .get_raw_callable_identifiers(user_component_id);
        let callable_type = component_db.user_component_db()[user_component_id].callable_type();
        let location = component_db
            .user_component_db()
            .get_location(user_component_id);
        let source = match location.source_file(package_graph) {
            Ok(s) => s,
            Err(e) => {
                diagnostics.push(e.into());
                continue;
            }
        };
        let source_span = diagnostic::get_f_macro_invocation_span(&source, location);
        let error = anyhow!(
            "Route parameters must be extracted using a plain struct with named fields, \
            where the name of each field matches one of the route parameters specified \
            in the route for the respective request handler.\n\
            `{}` is trying to extract `RouteParams<{extracted_type:?}>`, but \
            {error_suffix}, not a plain struct type. I don't support this: the extraction would \
            fail at runtime, when trying to process an incoming request.",
            raw_identifiers.fully_qualified_path().join("::")
        );
        let d = CompilerDiagnostic::builder(source, error)
            .optional_label(source_span.labeled(format!(
                "The {callable_type} asking for `RouteParams<{extracted_type:?}>`"
            )))
            .help(
                "Use a plain struct with named fields to extract route parameters.\n\
                Check out `RouteParams`' documentation for all the details!"
                    .into(),
            )
            .build();
        diagnostics.push(d.into());
    }
    Err(())
}

/// Return the set of user component ids that consume a certain instance of the `RouteParams` extractor
/// as input parameter.
fn route_params_consumer_ids(
    component_db: &ComponentDb,
    call_graph: &RawCallGraph,
    ok_route_params_node_id: NodeIndex,
) -> IndexSet<UserComponentId> {
    let mut consumer_ids = IndexSet::new();
    let mut descendant_ids = call_graph
        .neighbors_directed(ok_route_params_node_id, Direction::Outgoing)
        .collect::<IndexSet<_>>();
    while let Some(descendant_id) = descendant_ids.pop() {
        let descendant_node = &call_graph[descendant_id];
        if let CallGraphNode::Compute { component_id, .. } = descendant_node {
            if let Some(user_component_id) = component_db.user_component_id(*component_id) {
                consumer_ids.insert(user_component_id);
            }
        }
    }
    consumer_ids
}
