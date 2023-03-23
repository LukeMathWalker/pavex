use std::str::FromStr;

use anyhow::anyhow;
use guppy::graph::PackageGraph;
use indexmap::{IndexMap, IndexSet};
use petgraph::Direction;
use proc_macro2::TokenStream;
use rustdoc_types::{ItemEnum, StructKind};
use syn::parse::Parser;
use syn::{parse_macro_input, Attribute, Meta};

use crate::compiler::analyses::call_graph::{CallGraph, CallGraphNode};
use crate::compiler::analyses::components::{ComponentDb, HydratedComponent};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::user_components::RouterKey;
use crate::compiler::computation::{Computation, MatchResultVariant};
use crate::compiler::constructors::Constructor;
use crate::diagnostic;
use crate::diagnostic::{CompilerDiagnostic, LocationExt, OptionalSourceSpanExt};
use crate::language::{GenericArgument, ResolvedType};
use crate::rustdoc::{CrateCollection, GlobalItemId};

/// For each handler, check if route parameters are extracted from the URL of the incoming request.
/// If so, check that the type of the route parameter is a struct with named fields and
/// that each named field maps to a route parameter for the corresponding handler.
pub(crate) fn verify_route_parameters(
    handler_call_graphs: &IndexMap<RouterKey, CallGraph>,
    computation_db: &ComputationDb,
    component_db: &ComponentDb,
    package_graph: &PackageGraph,
    krate_collection: &CrateCollection,
    diagnostics: &mut Vec<miette::Error>,
) {
    for (_router_key, call_graph) in handler_call_graphs {
        let Some((ok_route_params_node_id, ok_route_params_component_id, ty_)) = call_graph.call_graph.node_indices().find_map(|node_id| {
            let node = &call_graph.call_graph[node_id];
            let CallGraphNode::Compute { component_id, .. } = node else { return None; };
            let hydrated_component = component_db.hydrated_component(*component_id, computation_db);
            let HydratedComponent::Constructor(Constructor(Computation::MatchResult(m))) =
                hydrated_component else { return None; };
            if m.variant != MatchResultVariant::Ok {
                return None;
            }
            let ResolvedType::ResolvedPath(ty_) = &m.output else { return None; };
            if ty_.base_type == vec!["pavex_runtime", "extract", "route", "RouteParams"] {
                Some((node_id, component_id, ty_.clone()))
            } else {
                None
            }
        }) else { continue; };

        let GenericArgument::TypeParameter(extracted_type) = &ty_.generic_arguments[0] else { unreachable!() };
        let error_suffix = match extracted_type {
            ResolvedType::ResolvedPath(t) => {
                if let Some(item_id) = t.rustdoc_id.clone() {
                    let item = krate_collection.get_type_by_global_type_id(&GlobalItemId {
                        rustdoc_item_id: item_id,
                        package_id: t.package_id.clone(),
                    });
                    match item.inner {
                        ItemEnum::Union(_) => Some(format!("`{t:?}` is an union")),
                        ItemEnum::Enum(_) => Some(format!("`{t:?}` is an enum")),
                        ItemEnum::Struct(ref s) => match &s.kind {
                            StructKind::Unit => Some(format!(
                                "`{t:?}` is a struct with no fields (a.k.a. unit struct)"
                            )),
                            StructKind::Tuple(_) => Some(format!("`{t:?}` is a tuple struct")),
                            StructKind::Plain { .. } => None,
                        },
                        _ => None,
                    }
                } else {
                    None
                }
            }
            ResolvedType::Reference(r) => Some(format!("`{r:?}` is a reference")),
            ResolvedType::Tuple(t) => Some(format!("`{t:?}` is a tuple")),
            ResolvedType::ScalarPrimitive(s) => Some(format!("`{s:?}` is a primitive")),
            ResolvedType::Slice(s) => Some(format!("`{s:?}` is a slice")),
            ResolvedType::Generic(_) => {
                unreachable!()
            }
        };

        if let Some(error_suffix) = error_suffix {
            // Find the compute nodes that consume the `RouteParams` extractor and report
            // an error on each of them.
            let mut affected_ids = IndexSet::new();
            let mut descendant_ids = call_graph
                .call_graph
                .neighbors_directed(ok_route_params_node_id, Direction::Outgoing)
                .collect::<IndexSet<_>>();
            let borrow_component_id = component_db.borrow_id(*ok_route_params_component_id);
            while let Some(descendant_id) = descendant_ids.pop() {
                let descendant_node = &call_graph.call_graph[descendant_id];
                if let CallGraphNode::Compute { component_id, .. } = descendant_node {
                    if Some(component_id) == borrow_component_id.as_ref() {
                        descendant_ids.extend(
                            call_graph
                                .call_graph
                                .neighbors_directed(descendant_id, Direction::Outgoing),
                        );
                        continue;
                    }
                    if let Some(user_component_id) = component_db.user_component_id(*component_id) {
                        affected_ids.insert(user_component_id);
                    }
                }
            }

            for user_component_id in affected_ids {
                let raw_identifiers = component_db
                    .user_component_db
                    .get_raw_callable_identifiers(user_component_id);
                let callable_type =
                    component_db.user_component_db[user_component_id].callable_type();
                let location = component_db
                    .user_component_db
                    .get_location(user_component_id);
                let source = match location.source_file(package_graph) {
                    Ok(s) => s,
                    Err(e) => {
                        diagnostics.push(e.into());
                        return;
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
        }
    }
}
