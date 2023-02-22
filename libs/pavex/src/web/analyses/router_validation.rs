use anyhow::anyhow;
use guppy::graph::PackageGraph;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;

use crate::diagnostic;
use crate::diagnostic::{CompilerDiagnostic, LocationExt, SourceSpanExt, ZeroBasedOrdinal};
use crate::web::analyses::raw_identifiers::RawCallableIdentifiersDb;
use crate::web::analyses::user_components::{UserComponent, UserComponentDb, UserComponentId};

/// Examine the registered paths and methods guards to make sure that we do not
/// have any conflicts.
pub(crate) fn validate_router(
    user_component_db: &UserComponentDb,
    raw_identifiers_db: &RawCallableIdentifiersDb,
    package_graph: &PackageGraph,
    diagnostics: &mut Vec<miette::Error>,
) {
    let methods = [
        "GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS", "CONNECT", "TRACE",
    ];
    let mut path2method2component_id = IndexMap::<_, Vec<_>>::new();
    for (id, component) in user_component_db.iter() {
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
            // We do not want to return an error if the _same_ callable is being registered
            // as a request handler for the same path+method multiple times.
            let unique_handlers = relevant_handler_ids
                .iter()
                .unique_by(|id| user_component_db[**id].raw_callable_identifiers_id())
                .collect::<Vec<_>>();
            if unique_handlers.len() > 1 {
                push_router_conflict_diagnostic(
                    path,
                    method,
                    &unique_handlers,
                    user_component_db,
                    raw_identifiers_db,
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
    user_component_ids: &[&UserComponentId],
    user_component_db: &UserComponentDb,
    raw_identifiers_db: &RawCallableIdentifiersDb,
    package_graph: &PackageGraph,
    diagnostics: &mut Vec<miette::Error>,
) {
    let n_unique_handlers = user_component_ids.len();
    let mut sub_diagnostics = Vec::with_capacity(n_unique_handlers);
    for (i, user_component_id) in user_component_ids.iter().enumerate() {
        let user_component = &user_component_db[**user_component_id];
        let raw_identifier_id = user_component.raw_callable_identifiers_id();
        let location = raw_identifiers_db.get_location(raw_identifier_id);
        let source = match location.source_file(package_graph) {
            Ok(s) => s,
            Err(e) => {
                diagnostics.push(e.into());
                return;
            }
        };
        let label = diagnostic::get_f_macro_invocation_span(&source, location)
            .map(|s| s.labeled(format!("The {} conflicting handler", ZeroBasedOrdinal(i))));
        let diagnostic =
            CompilerDiagnostic::builder(source, anyhow::anyhow!("")).optional_label(label);
        sub_diagnostics.push(diagnostic);
    }
    let mut sub_diagnostics = sub_diagnostics.into_iter();
    let mut overall = sub_diagnostics.next().unwrap().error(
        anyhow!(
            "I do not know how to route incoming `{method} {path}` requests: you have registered {n_unique_handlers} \
            different request handlers for this path+method combination.",
        )
    ).help(
        "You can only register one request handler for each path+method combination. \
        Remove all but one of the conflicting request handlers.".into());
    for diagnostic in sub_diagnostics {
        overall = overall.related_error(diagnostic.build());
    }
    diagnostics.push(overall.build().into());
}
