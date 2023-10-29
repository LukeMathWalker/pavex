use std::collections::BTreeMap;

use anyhow::anyhow;
use bimap::BiHashMap;
use guppy::graph::PackageGraph;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use matchit::InsertError;

use crate::compiler::analyses::user_components::raw_db::RawUserComponentDb;
use crate::compiler::analyses::user_components::{
    ScopeGraph, ScopeId, UserComponent, UserComponentId,
};
use crate::diagnostic;
use crate::diagnostic::{
    AnnotatedSnippet, CompilerDiagnostic, LocationExt, OptionalSourceSpanExt, SourceSpanExt,
    ZeroBasedOrdinal,
};

pub(crate) struct Router {
    #[allow(dead_code)]
    path_router: matchit::Router<()>,
}

impl Router {
    pub(super) fn new(
        raw_user_component_db: &RawUserComponentDb,
        scope_graph: &ScopeGraph,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Result<Self, ()> {
        let path_router =
            Self::detect_path_conflicts(raw_user_component_db, package_graph, diagnostics)?;
        let _fallback_router = Self::fallback_router(
            path_router.clone(),
            raw_user_component_db,
            scope_graph,
            package_graph,
            diagnostics,
        )?;

        Ok(Self { path_router })
    }

    fn detect_path_conflicts(
        raw_user_component_db: &RawUserComponentDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Result<matchit::Router<()>, ()> {
        let mut path_router = matchit::Router::new();
        let mut errored = false;
        for (id, component) in raw_user_component_db.iter() {
            let UserComponent::RequestHandler { router_key, .. } = component else {
                continue;
            };
            let Err(e) = path_router.insert(router_key.path.clone(), ()) else {
                continue;
            };
            use matchit::InsertError::*;
            match e {
                // You can register the same path multiple times with
                // multiple methods. Method conflicts are handled elsewhere.
                // We have an issue if **different** paths conflict!
                Conflict { with } if with == router_key.path => {}
                Conflict { .. } | TooManyParams | UnnamedParam | InvalidCatchAll | _ => {
                    errored = true;
                    push_matchit_diagnostic(
                        &raw_user_component_db,
                        id,
                        e,
                        package_graph,
                        diagnostics,
                    );
                }
            }
        }
        if errored {
            Err(())
        } else {
            Ok(path_router)
        }
    }

    /// Determine, for each request handler, which fallback should be used if a request matches
    /// a registered path but doesn't match any of the user-registered method guards for that
    /// path.
    fn fallback_router(
        mut validation_router: matchit::Router<()>,
        raw_user_component_db: &RawUserComponentDb,
        scope_graph: &ScopeGraph,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Result<BTreeMap<UserComponentId, Option<UserComponentId>>, ()> {
        let n_diagnostics = diagnostics.len();

        // For every scope, there is at most one fallback.
        let scope_id2fallback_id = {
            let mut scope_id2fallback_id = BiHashMap::new();
            for (id, component) in raw_user_component_db.iter() {
                let UserComponent::Fallback { scope_id, .. } = component else {
                    continue;
                };
                let parents = scope_id.direct_parent_ids(scope_graph);
                assert_eq!(parents.len(), 1, "Fallbacks are always encapsulated in their own sub-scope and should only have one parent scope.");
                let parent_scope_id = parents.into_iter().next().unwrap();
                // This is the root scope, we don't need to check anything.
                // We'll use that fallback for routes that fail to match.
                scope_id2fallback_id.insert(parent_scope_id, id);
            }
            scope_id2fallback_id
        };

        let scope_based_fallback_router = FallbackTree::new(&scope_id2fallback_id, scope_graph);

        let mut path_based_fallback_router = matchit::Router::new();
        'outer: for (fallback_id, component) in raw_user_component_db.iter() {
            let UserComponent::Fallback { .. } = component else {
                continue;
            };
            let path_prefix = &raw_user_component_db.fallback_id2path_prefix[&fallback_id];
            // If there is a nested blueprint with a path prefix, we register a path-based fallback
            // for all incoming requests that match that prefix.
            if let Some(path_prefix) = path_prefix {
                let trailing_capture = prefix_ends_with_capture(path_prefix);
                let fallback_path = match trailing_capture {
                    None => {
                        format!("{path_prefix}*catch_all")
                    }
                    Some(Capture::CatchAll(_)) => {
                        continue 'outer;
                    }
                    Some(Capture::Parameter(param)) => {
                        let stripped = path_prefix.strip_suffix(&param).unwrap();
                        format!("{stripped}*catch_all")
                    }
                };
                if let Err(e) = validation_router.insert(fallback_path.clone(), ()) {
                    if let InsertError::Conflict { .. } = e {
                        // There is already a user-registered route that serves as catch-all
                        // therefore we don't need to actually register this fallback.
                        // TODO: should we warn the user about this?
                        continue;
                    } else {
                        unreachable!()
                    }
                }
                path_based_fallback_router
                    .insert(fallback_path, fallback_id)
                    .unwrap();
            }
        }

        let mut handler_id2fallback_id = BTreeMap::new();
        // We now iterate over all request handlers to verify that path-based and scope-based
        // fallbacks match.
        // If they don't, we emit a diagnostic: there is ambiguity in the routing logic and we
        // don't know which fallback to use.
        for (handler_id, component) in raw_user_component_db.iter() {
            let UserComponent::RequestHandler {
                router_key,
                scope_id,
                ..
            } = component
            else {
                continue;
            };

            let path_fallback = path_based_fallback_router
                .at(router_key.path.as_str())
                .ok()
                .map(|m| m.value)
                .copied();
            let scope_fallback_id =
                scope_based_fallback_router.find_fallback_id(*scope_id, scope_graph);
            match path_fallback {
                None => {
                    // Good: there wasn't any path-based fallback, so it's all down to
                    // to the scope the route was registered against.
                    handler_id2fallback_id.insert(handler_id, scope_fallback_id);
                }
                Some(path_fallback_id) => {
                    if Some(path_fallback_id) != scope_fallback_id {
                        push_fallback_ambiguity_diagnostic(
                            raw_user_component_db,
                            scope_fallback_id,
                            path_fallback_id,
                            handler_id,
                            package_graph,
                            diagnostics,
                        );
                    } else {
                        // Good: they both use the same fallback.
                        handler_id2fallback_id.insert(handler_id, Some(path_fallback_id));
                    }
                }
            }
        }

        if n_diagnostics == diagnostics.len() {
            Ok(handler_id2fallback_id)
        } else {
            Err(())
        }
    }
}

fn prefix_ends_with_capture(path: &str) -> Option<Capture> {
    // Prefixes, if not empty, **cannot** end with a `/`.
    // Therefore we will always get a `Some` from `split_last` and it won't be empty.
    let last_segment = path.split('/').last().unwrap();
    if let Some((_, param)) = last_segment.split_once(':') {
        Some(Capture::Parameter(param.to_owned()))
    } else if let Some((_, param)) = last_segment.split_once('*') {
        Some(Capture::CatchAll(param.to_owned()))
    } else {
        None
    }
}

/// A tree that contains a node for each registered fallback (as well as the default one, if needed).
///
/// The tree is built by traversing the scope graph and for each scope that has a fallback, we
/// register a node in the tree.
/// A node is a child of another node if the scope it represents is a descendant of the scope of
/// the parent node.
struct FallbackTree {
    nodes: Vec<FallbackNode>,
}

impl FallbackTree {
    fn new(
        scope_id2fallback_id: &BiHashMap<ScopeId, UserComponentId>,
        scope_graph: &ScopeGraph,
    ) -> Self {
        let root = FallbackNode {
            scope_id: scope_graph.root_scope_id(),
            fallback_id: scope_id2fallback_id
                .get_by_left(&scope_graph.root_scope_id())
                .copied(),
            children_ids: vec![],
        };
        let mut stack: Vec<_> = root
            .scope_id
            .direct_children_ids(scope_graph)
            .into_iter()
            .map(|id| (id, 0))
            .collect();
        let mut nodes = vec![root];

        while let Some((scope_id, parent_node_index)) = stack.pop() {
            let parent_node_index =
                if let Some(fallback_id) = scope_id2fallback_id.get_by_left(&scope_id) {
                    let node = FallbackNode {
                        scope_id,
                        fallback_id: Some(*fallback_id),
                        children_ids: Vec::new(),
                    };
                    let node_index = nodes.len();
                    nodes[parent_node_index].children_ids.push(node_index);
                    nodes.push(node);
                    node_index
                } else {
                    parent_node_index
                };
            for child_scope_id in scope_id.direct_children_ids(scope_graph) {
                if child_scope_id == scope_graph.application_state_scope_id() {
                    continue;
                }
                stack.push((child_scope_id, parent_node_index));
            }
        }

        Self { nodes }
    }

    /// Return the root node of the fallback graph.
    fn root(&self) -> &FallbackNode {
        &self.nodes[0]
    }

    /// Find the id of the fallback handler that should be used for a given route
    /// based on the scope hierarchy.
    fn find_fallback_id(
        &self,
        route_scope_id: ScopeId,
        scope_graph: &ScopeGraph,
    ) -> Option<UserComponentId> {
        let mut current: &FallbackNode = self.root();
        'outer: loop {
            if current.scope_id == route_scope_id {
                return current.fallback_id;
            }
            for child_index in &current.children_ids {
                let child = &self.nodes[*child_index];
                if route_scope_id.is_descendant_of(child.scope_id, scope_graph) {
                    current = child;
                    continue 'outer;
                }
            }
            return current.fallback_id;
        }
    }
}

struct FallbackNode {
    scope_id: ScopeId,
    // The user may or may not have replaced the framework default fallback handler,
    // hence the `Option` here.
    fallback_id: Option<UserComponentId>,
    children_ids: Vec<usize>,
}

enum Capture {
    /// E.g. `*foo`
    CatchAll(String),
    /// E.g. `:foo`
    Parameter(String),
}

/// Examine the registered paths and methods guards to make sure that we don't
/// have any conflicts—i.e. multiple handlers registered for the same path+method combination.
pub(super) fn build_router(
    raw_user_component_db: &RawUserComponentDb,
    scope_graph: &ScopeGraph,
    package_graph: &PackageGraph,
    diagnostics: &mut Vec<miette::Error>,
) {
    detect_method_conflicts(raw_user_component_db, package_graph, diagnostics);
    let _ = Router::new(
        raw_user_component_db,
        scope_graph,
        package_graph,
        diagnostics,
    );
}

/// Examine the registered paths and methods guards to make sure that we don't
/// have any conflicts—i.e. multiple handlers registered for the same path+method combination.
fn detect_method_conflicts(
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

fn push_fallback_ambiguity_diagnostic(
    raw_user_component_db: &RawUserComponentDb,
    scope_fallback_id: Option<UserComponentId>,
    path_fallback_id: UserComponentId,
    route_id: UserComponentId,
    package_graph: &PackageGraph,
    diagnostics: &mut Vec<miette::Error>,
) {
    let UserComponent::RequestHandler { router_key, .. } = &raw_user_component_db[route_id] else {
        unreachable!()
    };
    let route_location = raw_user_component_db.get_location(route_id);
    let route_source = match route_location.source_file(package_graph) {
        Ok(s) => s,
        Err(e) => {
            diagnostics.push(e.into());
            return;
        }
    };
    let label = diagnostic::get_route_path_span(&route_source, &route_location)
        .labeled("The route was registered here".to_string());
    let route_repr = router_key.diagnostic_repr();
    let scope_fallback = match scope_fallback_id {
        None => "the default framework fallback".to_string(),
        Some(fallback_id) => {
            let UserComponent::Fallback {
                raw_callable_identifiers_id,
                ..
            } = &raw_user_component_db[fallback_id]
            else {
                unreachable!()
            };
            format!(
                "`{}`",
                raw_user_component_db.identifiers_interner[*raw_callable_identifiers_id]
                    .fully_qualified_path()
                    .join("::")
            )
        }
    };
    let path_fallback = {
        let UserComponent::Fallback {
            raw_callable_identifiers_id,
            ..
        } = &raw_user_component_db[path_fallback_id]
        else {
            unreachable!()
        };
        raw_user_component_db.identifiers_interner[*raw_callable_identifiers_id]
            .fully_qualified_path()
            .join("::")
    };
    let path_prefix = raw_user_component_db.fallback_id2path_prefix[&path_fallback_id]
        .as_ref()
        .unwrap();
    let error = anyhow::anyhow!(
        "Routing logic can't be ambiguous.\n\
        You registered `{path_fallback}` as the fallback handler for all unmatched incoming requests \
        with a path that begins in `{path_prefix}`.\n\
        But `{route_repr}` wasn't registered against that blueprint!\n\
        It was registered under a different blueprint, with a different fallback handler: {scope_fallback}.\n\
        I can't determine which fallback is the most appropriate one for incoming `{}` requests \
        with a method that doesn't match the ones you registered a handler for.",
        router_key.path
    );
    let diagnostic = CompilerDiagnostic::builder(route_source, error)
        .optional_label(label)
        .help(format!(
            "You can fix this by registering `{route_repr}` against the nested blueprint \
            with `{path_prefix}` as prefix. All `{path_prefix}`-prefixed routes would then \
            be using `{path_fallback}` as fallback."
        ));
    diagnostics.push(diagnostic.build().into());
}

fn push_matchit_diagnostic(
    raw_user_component_db: &RawUserComponentDb,
    raw_user_component_id: UserComponentId,
    error: matchit::InsertError,
    package_graph: &PackageGraph,
    diagnostics: &mut Vec<miette::Error>,
) {
    // We want to control the error message for style consistency with the rest of the
    // diagnostics we emit.
    let error = match error {
        InsertError::Conflict { with } => {
            anyhow!("This route path conflicts with the path of another route you already registered, `{}`.", with)
        }
        InsertError::TooManyParams => {
            anyhow!("You can only register one route parameter per each path segment.")
        }
        InsertError::UnnamedParam => {
            anyhow!("All route parameters must be named. You can't use anonymous parameters like `:` or `*`.")
        }
        InsertError::InvalidCatchAll => {
            anyhow!("You can only use catch-all parameters at the end of a route path.")
        }
        _ => error.into(),
    };

    let location = raw_user_component_db.get_location(raw_user_component_id);
    let source = match location.source_file(package_graph) {
        Ok(s) => s,
        Err(e) => {
            diagnostics.push(e.into());
            return;
        }
    };
    let label = diagnostic::get_route_path_span(&source, location)
        .labeled("The problematic path".to_string());
    let diagnostic = CompilerDiagnostic::builder(source, error).optional_label(label);
    diagnostics.push(diagnostic.build().into());
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
