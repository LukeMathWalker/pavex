use anyhow::anyhow;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use petgraph::Direction;
use petgraph::graph::NodeIndex;
use rustdoc_types::{ItemEnum, StructKind};

use crate::compiler::analyses::call_graph::{CallGraphNode, RawCallGraph};
use crate::compiler::analyses::components::HydratedComponent;
use crate::compiler::analyses::components::{ComponentDb, ComponentId};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::processing_pipeline::RequestHandlerPipeline;
use crate::compiler::analyses::router::Router;
use crate::compiler::component::Constructor;
use crate::compiler::computation::{Computation, MatchResultVariant};
use crate::compiler::utils::process_framework_path;
use crate::diagnostic::{self, DiagnosticSink, OptionalLabeledSpanExt};
use crate::diagnostic::{CompilerDiagnostic, OptionalSourceSpanExt};
use crate::language::{GenericArgument, ResolvedType};
use crate::rustdoc::{CrateCollection, GlobalItemId};
use crate::utils::comma_separated_list;

use super::analyses::route_path::RoutePath;
use super::traits::assert_trait_is_implemented;

/// For each handler, check if path parameters are extracted from the URL of the incoming request.
/// If so, check that the type of the path parameter is a struct with named fields and
/// that each named field maps to a path parameter for the corresponding handler.
#[tracing::instrument(name = "Verify path parameters", skip_all)]
pub(crate) fn verify_path_parameters(
    router: &Router,
    handler_id2pipeline: &IndexMap<ComponentId, RequestHandlerPipeline>,
    computation_db: &ComputationDb,
    component_db: &ComponentDb,
    krate_collection: &CrateCollection,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) {
    let ResolvedType::ResolvedPath(structural_deserialize) = process_framework_path(
        "pavex::serialization::StructuralDeserialize",
        krate_collection,
    ) else {
        unreachable!()
    };

    let infos = router.route_infos();
    for handler_id in router.handler_ids() {
        let Some(pipeline) = handler_id2pipeline.get(&handler_id) else {
            continue;
        };

        // If `PathParams` is used, it will appear as a `Compute` node in *at most* one of the
        // call graphs in the processing pipeline for the handler, since it's a `RequestScoped`
        // component.
        let Some((graph, ok_path_params_node_id, ty_)) = pipeline.graph_iter().find_map(|graph| {
            let graph = &graph.call_graph;
            graph
                .node_indices()
                .find_map(|node_id| {
                    let node = &graph[node_id];
                    let CallGraphNode::Compute { component_id, .. } = node else {
                        return None;
                    };
                    let hydrated_component =
                        component_db.hydrated_component(*component_id, computation_db);
                    let HydratedComponent::Constructor(Constructor(Computation::MatchResult(m))) =
                        hydrated_component
                    else {
                        return None;
                    };
                    if m.variant != MatchResultVariant::Ok {
                        return None;
                    }
                    let ResolvedType::ResolvedPath(ty_) = &m.output else {
                        return None;
                    };
                    if ty_.base_type == vec!["pavex", "request", "path", "PathParams"] {
                        Some((node_id, ty_.clone()))
                    } else {
                        None
                    }
                })
                .map(|(node_id, ty_)| (graph, node_id, ty_))
        }) else {
            continue;
        };

        let GenericArgument::TypeParameter(extracted_type) = &ty_.generic_arguments[0] else {
            unreachable!()
        };

        let Ok(struct_item) = must_be_a_plain_struct(
            component_db,
            computation_db,
            krate_collection,
            diagnostics,
            graph,
            ok_path_params_node_id,
            extracted_type,
        ) else {
            continue;
        };
        let ItemEnum::Struct(struct_inner_item) = &struct_item.inner else {
            unreachable!()
        };

        // We only want to check alignment between struct fields and the path parameters in the
        // template if the struct implements `StructuralDeserialize`, our marker trait that stands
        // for "this struct implements serde::Deserialize using a #[derive(serde::Deserialize)] with
        // no customizations (e.g. renames)".
        if assert_trait_is_implemented(krate_collection, extracted_type, &structural_deserialize)
            .is_err()
        {
            continue;
        }

        let path = &infos[handler_id].path;
        let parsed_path = RoutePath::parse(path.to_owned());
        let path_parameter_names = parsed_path
            .parameters
            .keys()
            .cloned()
            .collect::<IndexSet<_>>();
        let struct_field_names = {
            let mut struct_field_names = IndexSet::new();
            let ResolvedType::ResolvedPath(extracted_path_type) = &extracted_type else {
                unreachable!()
            };
            let StructKind::Plain {
                fields: field_ids, ..
            } = &struct_inner_item.kind
            else {
                unreachable!()
            };
            for field_id in field_ids {
                let field_item = krate_collection.get_item_by_global_type_id(&GlobalItemId {
                    rustdoc_item_id: *field_id,
                    package_id: extracted_path_type.package_id.clone(),
                });
                struct_field_names.insert(field_item.name.clone().unwrap());
            }
            struct_field_names
        };

        let non_existing_path_parameters = struct_field_names
            .into_iter()
            .filter(|f| !path_parameter_names.contains(f.as_str()))
            .collect::<IndexSet<_>>();

        if !non_existing_path_parameters.is_empty() {
            report_non_existing_path_parameters(
                component_db,
                computation_db,
                diagnostics,
                path,
                graph,
                ok_path_params_node_id,
                path_parameter_names,
                non_existing_path_parameters,
                extracted_type,
            )
        }
    }
}

/// Report an error on each compute node that consumes the `PathParams` extractor
/// while trying to extract one or more path parameters that are not present in
/// the respective path pattern.
fn report_non_existing_path_parameters(
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
    diagnostics: &mut DiagnosticSink,
    path: &str,
    call_graph: &RawCallGraph,
    ok_path_params_node_id: NodeIndex,
    path_parameter_names: IndexSet<String>,
    non_existing_path_parameters: IndexSet<String>,
    extracted_type: &ResolvedType,
) {
    assert!(!non_existing_path_parameters.is_empty());
    // Find the compute nodes that consume the `PathParams` extractor and report
    // an error on each of them.
    let consuming_ids = path_params_consumer_ids(call_graph, ok_path_params_node_id);
    for component_id in consuming_ids {
        let Some(user_component_id) = component_db.user_component_id(component_id) else {
            continue;
        };
        let callable = &computation_db[user_component_id];
        let location = component_db
            .user_component_db()
            .get_location(user_component_id);
        let source = diagnostics.source(location).map(|s| {
            diagnostic::f_macro_span(s.source(), location)
                .labeled(format!(
                    "The {} asking for `PathParams<{extracted_type:?}>`",
                    component_db.user_component_db()[user_component_id].kind()
                ))
                .attach(s)
        });

        if path_parameter_names.is_empty() {
            let error = anyhow!(
                "`{}` is trying to extract path parameters using `PathParams<{extracted_type:?}>`.\n\
                    But there are no path parameters in `{path}`, the corresponding path pattern!",
                callable.path
            );
            let d = CompilerDiagnostic::builder(error)
                .optional_source(source)
                .help(
                    "Stop trying to extract path parameters, or add them to the path pattern!"
                        .into(),
                )
                .build();
            diagnostics.push(d);
        } else {
            let missing_msg = if non_existing_path_parameters.len() == 1 {
                let name = non_existing_path_parameters.first().unwrap();
                format!(
                    "There is no path parameter named `{name}`, but there is a struct field named `{name}` \
                    in `{extracted_type:?}`"
                )
            } else {
                use std::fmt::Write;

                let mut msg = "There are no path parameters named ".to_string();
                comma_separated_list(
                    &mut msg,
                    non_existing_path_parameters.iter(),
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
            let path_parameters = path_parameter_names
                .iter()
                .map(|p| format!("- `{}`", p))
                .join("\n");
            let error = anyhow!(
                "`{}` is trying to extract path parameters using `PathParams<{extracted_type:?}>`.\n\
                    Every struct field in `{extracted_type:?}` must be named after one of the route \
                    parameters that appear in `{path}`:\n{path_parameters}\n\n\
                    {missing_msg}. This is going to cause a runtime error!",
                callable.path,
            );
            let d = CompilerDiagnostic::builder(error)
                .optional_source(source)
                .help(
                    "Remove or rename the fields that do not map to a valid path parameter.".into(),
                )
                .build();
            diagnostics.push(d);
        }
    }
}

/// Checks that the type of the path parameter is a struct with named fields.
/// If it is, returns the rustdoc item for the type.
/// If it isn't, reports an error diagnostic on each compute node that consumes the
/// `PathParams` extractor.
fn must_be_a_plain_struct(
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
    krate_collection: &CrateCollection,
    diagnostics: &mut DiagnosticSink,
    call_graph: &RawCallGraph,
    ok_path_params_node_id: NodeIndex,
    extracted_type: &ResolvedType,
) -> Result<rustdoc_types::Item, ()> {
    let error_suffix = match extracted_type {
        ResolvedType::ResolvedPath(t) => {
            let Some(item_id) = t.rustdoc_id else {
                unreachable!()
            };
            let item = krate_collection.get_item_by_global_type_id(&GlobalItemId {
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

    // Find the compute nodes that consume the `PathParams` extractor and report
    // an error on each of them.
    let consuming_ids = path_params_consumer_ids(call_graph, ok_path_params_node_id);

    for component_id in consuming_ids {
        let Some(user_component_id) = component_db.user_component_id(component_id) else {
            continue;
        };
        let callable = &computation_db[user_component_id];
        let callable_type = component_db.user_component_db()[user_component_id].kind();
        let location = component_db
            .user_component_db()
            .get_location(user_component_id);
        let source = diagnostics.source(location).map(|s| {
            diagnostic::f_macro_span(s.source(), location)
                .labeled(format!(
                    "The {callable_type} asking for `PathParams<{extracted_type:?}>`"
                ))
                .attach(s)
        });
        let error = anyhow!(
            "Path parameters must be extracted using a plain struct with named fields, \
            where the name of each field matches one of the path parameters specified \
            in the route for the respective request handler.\n\
            `{}` is trying to extract `PathParams<{extracted_type:?}>`, but \
            {error_suffix}, not a plain struct type. I don't support this: the extraction would \
            fail at runtime, when trying to process an incoming request.",
            callable.path
        );
        let d = CompilerDiagnostic::builder(error)
            .optional_source(source)
            .help(
                "Use a plain struct with named fields to extract path parameters.\n\
                Check out `PathParams`' documentation for all the details!"
                    .into(),
            )
            .build();
        diagnostics.push(d);
    }
    Err(())
}

/// Return the set of user component ids that consume a certain instance of the `PathParams` extractor
/// as input parameter.
fn path_params_consumer_ids(
    call_graph: &RawCallGraph,
    ok_path_params_node_id: NodeIndex,
) -> IndexSet<ComponentId> {
    let mut consumer_ids = IndexSet::new();
    let mut descendant_ids = call_graph
        .neighbors_directed(ok_path_params_node_id, Direction::Outgoing)
        .collect::<IndexSet<_>>();
    while let Some(descendant_id) = descendant_ids.pop() {
        let descendant_node = &call_graph[descendant_id];
        if let CallGraphNode::Compute { component_id, .. } = descendant_node {
            consumer_ids.insert(*component_id);
        }
    }
    consumer_ids
}
