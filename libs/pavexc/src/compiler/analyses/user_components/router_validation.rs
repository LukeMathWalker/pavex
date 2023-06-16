use anyhow::anyhow;
use guppy::graph::PackageGraph;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;

use crate::compiler::analyses::user_components::raw_db::RawUserComponentDb;
use crate::compiler::analyses::user_components::{UserComponent, UserComponentId};
use crate::diagnostic;
use crate::diagnostic::{
    AnnotatedSnippet, CompilerDiagnostic, LocationExt, SourceSpanExt, ZeroBasedOrdinal,
};

/// Examine the registered paths and methods guards to make sure that we don't
/// have any conflicts—i.e. multiple handlers registered for the same path+method combination.
pub(super) fn validate_router(
    raw_user_component_db: &RawUserComponentDb,
    package_graph: &PackageGraph,
    diagnostics: &mut Vec<miette::Error>,
) {
    let methods = [
        "GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS", "CONNECT", "TRACE",
    ];
    let mut path2method2component_id = IndexMap::<_, Vec<_>>::new();
    for (id, component) in raw_user_component_db.iter() {
        if let UserComponent::RequestHandler { router_key, .. } = component {
            path2method2component_id
                .entry(&router_key.path)
                .or_default()
                .push((&router_key.method_guard, id));
        }
    }

    for (path, routes) in path2method2component_id.into_iter() {
        for method in methods {
            let mut relevant_handler_ids = IndexSet::new();
            for (guard, id) in &routes {
                match guard {
                    // `None` stands for the `ANY` guard, it matches all methods
                    None => {
                        relevant_handler_ids.insert(*id);
                    }
                    Some(method_guards) => {
                        if method_guards.contains(method) {
                            relevant_handler_ids.insert(*id);
                        }
                    }
                }
            }
            // We don't want to return an error if the _same_ callable is being registered
            // as a request handler for the same path+method multiple times.
            let unique_handlers = relevant_handler_ids
                .iter()
                .unique_by(|id| raw_user_component_db[**id].raw_callable_identifiers_id())
                .collect::<Vec<_>>();
            if unique_handlers.len() > 1 {
                push_router_conflict_diagnostic(
                    path,
                    method,
                    &unique_handlers,
                    raw_user_component_db,
                    package_graph,
                    diagnostics,
                );
            }
        }
    }
}

fn push_router_conflict_diagnostic(
    path: &str,
    method: &str,
    raw_user_component_ids: &[&UserComponentId],
    raw_user_component_db: &RawUserComponentDb,
    package_graph: &PackageGraph,
    diagnostics: &mut Vec<miette::Error>,
) {
    let n_unique_handlers = raw_user_component_ids.len();
    let mut annotated_snippets: Vec<AnnotatedSnippet> = Vec::with_capacity(n_unique_handlers);
    for (i, raw_user_component_id) in raw_user_component_ids.iter().enumerate() {
        let location = raw_user_component_db.get_location(**raw_user_component_id);
        let source = match location.source_file(package_graph) {
            Ok(s) => s,
            Err(e) => {
                diagnostics.push(e.into());
                continue;
            }
        };
        if let Some(s) = diagnostic::get_f_macro_invocation_span(&source, location) {
            let label = s.labeled(format!("The {} conflicting handler", ZeroBasedOrdinal(i)));
            annotated_snippets.push(AnnotatedSnippet::new(source, label));
        }
    }
    let mut annotated_snippets = annotated_snippets.into_iter();
    let first = annotated_snippets.next().unwrap();
    let overall = CompilerDiagnostic::builder(first.source_code, anyhow!(
            "I don't know how to route incoming `{method} {path}` requests: you have registered {n_unique_handlers} \
            different request handlers for this path+method combination."
        ))
        .labels(first.labels.into_iter())
        .additional_annotated_snippets(annotated_snippets)
        .help(
            "You can only register one request handler for each path+method combination. \
            Remove all but one of the conflicting request handlers.".into()
        );
    diagnostics.push(overall.build().into());
}
