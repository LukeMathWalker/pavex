use std::collections::{BTreeMap, BTreeSet};

use ahash::{HashMap, HashMapExt};
use anyhow::anyhow;
use bimap::BiHashMap;
use guppy::graph::PackageGraph;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use matchit::InsertError;
use pavex_bp_schema::MethodGuard;

use crate::compiler::analyses::domain::DomainGuard;
use crate::compiler::analyses::route_path::RoutePath;
use crate::compiler::analyses::user_components::raw_db::RawUserComponentDb;
use crate::compiler::analyses::user_components::{
    ScopeGraph, ScopeId, UserComponent, UserComponentId,
};
use crate::diagnostic::{
    AnnotatedSnippet, CompilerDiagnostic, OptionalSourceSpanExt, SourceSpanExt, ZeroBasedOrdinal,
};
use crate::utils::comma_separated_list;
use crate::{diagnostic, try_source};

/// A mechanism to route incoming requests to the correct handler.
#[derive(Debug)]
pub(crate) enum Router {
    DomainAgnostic(PathRouter),
    DomainBased(DomainRouter),
}

impl Router {
    pub(super) fn new(
        raw_user_component_db: &RawUserComponentDb,
        scope_graph: &ScopeGraph,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Result<Self, ()> {
        let Ok(is_domain_based) = Router::is_domain_based(raw_user_component_db) else {
            Self::either_all_domain_based_or_all_agnostic(
                raw_user_component_db,
                package_graph,
                diagnostics,
            );
            return Err(());
        };

        // A global scope<>fallback mapping.
        let scope_based_fallback_tree = {
            // For every scope there is at most one fallback.
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
            ScopeBasedFallbackTree::new(&scope_id2fallback_id, scope_graph)
        };

        if is_domain_based {
            Ok(Router::DomainBased(DomainRouter::new(
                raw_user_component_db,
                scope_graph,
                &scope_based_fallback_tree,
                package_graph,
                diagnostics,
            )?))
        } else {
            let component_ids: Vec<_> = raw_user_component_db
                .iter()
                .filter_map(|(id, component)| {
                    if matches!(
                        component,
                        UserComponent::RequestHandler { .. } | UserComponent::Fallback { .. }
                    ) {
                        Some(id)
                    } else {
                        None
                    }
                })
                .collect();
            Ok(Router::DomainAgnostic(PathRouter::new(
                &component_ids,
                raw_user_component_db,
                scope_graph,
                &scope_based_fallback_tree,
                package_graph,
                diagnostics,
            )?))
        }
    }

    /// Returns `true` if all handlers have a domain guard, `false` if all handlers are domain agnostic.
    /// Returns `Err` if some handlers have a domain guard and some do not.
    fn is_domain_based(raw_user_component_db: &RawUserComponentDb) -> Result<bool, ()> {
        // Either all handlers have a domain guard, or none do.
        let mut any_domain_based = false;
        let mut any_domain_agnostic = false;
        for component in raw_user_component_db.components() {
            let UserComponent::RequestHandler { router_key, .. } = component else {
                continue;
            };

            any_domain_based |= router_key.domain_guard.is_some();
            any_domain_agnostic |= router_key.domain_guard.is_none();

            if any_domain_based && any_domain_agnostic {
                return Err(());
            }
        }
        Ok(any_domain_based)
    }

    fn either_all_domain_based_or_all_agnostic(
        raw_user_component_db: &RawUserComponentDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let e = anyhow::anyhow!(
            "When registering request handlers, you must make a choice: either all \
            handlers have a domain constraint, or none do.\n\
            Your application violates this rule: there are both domain-specific and domain-agnostic handlers."
        );
        let diagnostic = CompilerDiagnostic::builder(e).help(
            "To avoid routing ambiguity, you must either:\n- Add a domain guard to all handlers that \
                don't have one\n- Remove domain guards from all handlers that have one"
                .into(),
        );

        let domain_based_snippet = {
            let id = raw_user_component_db
                .iter()
                .find_map(|(id, component)| match component {
                    UserComponent::RequestHandler { router_key, .. } => {
                        if router_key.domain_guard.is_some() {
                            Some(id)
                        } else {
                            None
                        }
                    }
                    _ => None,
                })
                .unwrap();
            let location = raw_user_component_db.get_location(id);
            try_source!(location, package_graph, diagnostics).map(|source| {
                let label = diagnostic::get_route_path_span(&source, location)
                    .labeled("A handler restricted to a specific domain".to_string());
                AnnotatedSnippet::new_optional(source, label)
            })
        };
        let domain_agnostic_snippet = {
            let id = raw_user_component_db
                .iter()
                .find_map(|(id, component)| match component {
                    UserComponent::RequestHandler { router_key, .. } => {
                        if router_key.domain_guard.is_none() {
                            Some(id)
                        } else {
                            None
                        }
                    }
                    _ => None,
                })
                .unwrap();
            let location = raw_user_component_db.get_location(id);
            try_source!(location, package_graph, diagnostics).map(|source| {
                let label = diagnostic::get_route_path_span(&source, location)
                    .labeled("A handler without a domain restriction".to_string());
                AnnotatedSnippet::new_optional(source, label)
            })
        };

        let diagnostic = diagnostic
            .optional_additional_annotated_snippet(domain_based_snippet)
            .optional_additional_annotated_snippet(domain_agnostic_snippet)
            .build()
            .into();
        diagnostics.push(diagnostic);
    }
}

/// Route requests to handlers based on their domain, path, and HTTP method.
#[derive(Debug)]
pub(crate) struct DomainRouter {
    /// A map from the domain to the path router for that domain.
    pub(crate) domain2path_router: BTreeMap<DomainGuard, PathRouter>,
    /// The fallback to use if the domain of the incoming request doesn't match any of the domains
    /// we are looking for.
    pub(crate) root_fallback_id: UserComponentId,
}

/// Route requests to handlers based on their path and HTTP method.
#[derive(Debug)]
pub(crate) struct PathRouter {
    /// A map from the path to the HTTP methods that it can handle.
    pub(crate) path2method_router: BTreeMap<String, LeafRouter>,
    /// The fallback to use if no route matches the incoming request.
    pub(crate) root_fallback_id: UserComponentId,
}

/// A router to dispatch a request to a handler based on its method, after having matched its path.
#[derive(Debug, Clone)]
pub(crate) struct LeafRouter {
    // TODO: we could use a more memory efficient representation here (e.g. a bitset) to describe
    //     the set of methods that a handler can handle.
    pub(crate) handler_id2methods: BTreeMap<UserComponentId, BTreeSet<String>>,
    /// The fallback to use if the method of the incoming request doesn't match any of the
    /// methods registered for the route.
    /// We always need a fallback, since you might receive requests with "non-standard" methods.
    pub(crate) fallback_id: UserComponentId,
}

impl DomainRouter {
    fn new(
        db: &RawUserComponentDb,
        scope_graph: &ScopeGraph,
        scope_based_fallback_tree: &ScopeBasedFallbackTree,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Result<Self, ()> {
        let (domain2components, root_fallback_id) = {
            let mut domain2components: BTreeMap<_, Vec<_>> = Default::default();
            let mut root_fallback_id = None;
            for (id, component) in db.iter() {
                match component {
                    UserComponent::RequestHandler { router_key, .. } => {
                        // Safe to unwrap because we've already checked that all handlers are domain-specific.
                        let domain_guard = router_key.domain_guard.as_ref().unwrap();
                        if !domain2components.contains_key(domain_guard) {
                            domain2components.insert(domain_guard.clone(), vec![id]);
                        } else {
                            domain2components.get_mut(domain_guard).unwrap().push(id);
                        }
                    }
                    UserComponent::Fallback { .. } => {
                        let domain_guard = &db.fallback_id2domain_guard[&id];
                        match domain_guard {
                            Some(domain_guard) => {
                                if !domain2components.contains_key(domain_guard) {
                                    domain2components.insert(domain_guard.clone(), vec![id]);
                                } else {
                                    domain2components.get_mut(domain_guard).unwrap().push(id);
                                }
                            }
                            None => {
                                root_fallback_id = Some(id);
                            }
                        }
                    }
                    _ => {}
                }
            }
            let root_fallback_id = root_fallback_id.expect("There must always be a top-level fallback, either user-provided or framework-provided");
            (domain2components, root_fallback_id)
        };

        let mut domain2path_router = BTreeMap::new();
        for (domain, components) in domain2components {
            let path_router = PathRouter::new(
                &components,
                db,
                scope_graph,
                scope_based_fallback_tree,
                package_graph,
                diagnostics,
            )?;
            domain2path_router.insert(domain, path_router);
        }

        Self::detect_domain_conflicts(db, package_graph, diagnostics)?;

        Ok(Self {
            domain2path_router,
            root_fallback_id,
        })
    }

    /// Make sure that the user-registered domains don't conflict with each other.
    /// In other words: we won't encounter any issue when creating this router.
    ///
    /// How do we do that?
    ///
    /// By trying to create the router in the compiler itself!
    /// If it works now, it'll work at runtime too.
    fn detect_domain_conflicts(
        db: &RawUserComponentDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Result<(), ()> {
        let mut router = matchit::Router::new();
        let mut has_errored = false;
        let mut pattern2guard = HashMap::new();
        for guard in db.domain_guard2locations.keys() {
            let pattern = guard.matchit_pattern();
            pattern2guard.insert(pattern.clone(), guard);
            let Err(e) = router.insert(pattern, ()) else {
                continue;
            };
            has_errored = true;

            let matchit::InsertError::Conflict { with } = e else {
                unreachable!(
                    "All other domain guard errors should have been caught and reported by now"
                )
            };
            Self::push_domain_conflict_diagnostic(
                db,
                guard,
                pattern2guard[&with],
                package_graph,
                diagnostics,
            );
        }

        if has_errored {
            Err(())
        } else {
            Ok(())
        }
    }

    fn push_domain_conflict_diagnostic(
        db: &RawUserComponentDb,
        domain_1: &DomainGuard,
        domain_2: &DomainGuard,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let error = anyhow::anyhow!(
            "There is an overlap between two of the domain constraints you registered, `{}` and `{}`.\n\
            I wouldn't know where to route a request that matches both of them, since there's no clear priority between them.",
            domain_1,
            domain_2
        );

        let snippet1 = {
            let location = db.domain_guard2locations[domain_1].first().unwrap();
            let source = try_source!(location, package_graph, diagnostics);
            let label = source
                .as_ref()
                .map(|source| {
                    diagnostic::get_domain_span(&source, location)
                        .labeled("The first domain".to_string())
                })
                .flatten();
            source.map(|s| AnnotatedSnippet::new_optional(s, label))
        };
        let snippet2 = {
            let location = db.domain_guard2locations[domain_2].first().unwrap();
            let source = try_source!(location, package_graph, diagnostics);
            let label = source
                .as_ref()
                .map(|source| {
                    diagnostic::get_domain_span(&source, location)
                        .labeled("The second domain".to_string())
                })
                .flatten();
            source.map(|s| AnnotatedSnippet::new_optional(s, label))
        };
        let diagnostic = CompilerDiagnostic::builder(error)
            .optional_additional_annotated_snippet(snippet1)
            .optional_additional_annotated_snippet(snippet2)
            .help("Can you rewrite your domain constraints so that they don't overlap?".into());
        diagnostics.push(diagnostic.build().into());
    }
}

impl LeafRouter {
    pub fn new(fallback_id: UserComponentId) -> Self {
        Self {
            handler_id2methods: Default::default(),
            fallback_id,
        }
    }
}

impl PathRouter {
    fn new(
        component_ids: &[UserComponentId],
        db: &RawUserComponentDb,
        scope_graph: &ScopeGraph,
        scope_based_fallback_router: &ScopeBasedFallbackTree,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Result<Self, ()> {
        let root_scope_id = scope_graph
            .find_common_ancestor(component_ids.iter().map(|id| db[*id].scope_id()).collect());
        let root_fallback_id =
            scope_based_fallback_router.find_fallback_id(root_scope_id, scope_graph);

        Self::detect_method_conflicts(db, &component_ids, package_graph, diagnostics)?;
        let runtime_router =
            Self::detect_path_conflicts(db, &component_ids, package_graph, diagnostics)?;
        let (route_id2fallback_id, path_catchall2fallback_id) = Self::assign_fallbacks(
            runtime_router.clone(),
            &scope_based_fallback_router,
            &component_ids,
            db,
            scope_graph,
            package_graph,
            diagnostics,
        )?;
        Self::check_method_not_allowed_fallbacks(
            &route_id2fallback_id,
            &component_ids,
            db,
            package_graph,
            diagnostics,
        )?;

        let mut path2method_router = BTreeMap::new();
        for id in component_ids.iter() {
            let UserComponent::RequestHandler { router_key, .. } = &db[id] else {
                continue;
            };
            match &router_key.method_guard {
                MethodGuard::Any => {
                    // We don't need to register a fallback for this route, since it matches
                    // all methods.
                    path2method_router.insert(router_key.path.clone(), LeafRouter::new(*id));
                }
                MethodGuard::Some(methods) => {
                    let sub_router: &mut LeafRouter = path2method_router
                        .entry(router_key.path.clone())
                        .or_insert_with(|| LeafRouter::new(route_id2fallback_id[id]));
                    sub_router.handler_id2methods.insert(*id, methods.clone());
                }
            }
        }
        for (path, fallback_id) in path_catchall2fallback_id {
            path2method_router
                .entry(path)
                .or_insert_with(|| LeafRouter::new(fallback_id));
        }

        Ok(Self {
            root_fallback_id,
            path2method_router,
        })
    }

    /// Examine the registered paths and methods guards to make sure that we don't
    /// have any conflicts—i.e. multiple handlers registered for the same path+method combination.
    fn detect_method_conflicts(
        db: &RawUserComponentDb,
        component_ids: &[UserComponentId],
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Result<(), ()> {
        let n_diagnostics = diagnostics.len();

        let mut path2method2component_id = IndexMap::<_, Vec<_>>::new();
        for id in component_ids {
            let UserComponent::RequestHandler { router_key, .. } = &db[id] else {
                continue;
            };
            path2method2component_id
                .entry(&router_key.path)
                .or_default()
                .push((&router_key.method_guard, id));
        }

        for (path, routes) in path2method2component_id.into_iter() {
            for method in METHODS {
                let mut relevant_handler_ids = IndexSet::new();
                for (guard, &id) in &routes {
                    match guard {
                        // `None` stands for the `ANY` guard, it matches all well-known methods
                        MethodGuard::Any { .. } => {
                            relevant_handler_ids.insert(id);
                        }
                        MethodGuard::Some(method_guards) => {
                            if method_guards.contains(method) {
                                relevant_handler_ids.insert(id);
                            }
                        }
                    }
                }
                // We don't want to return an error if the _same_ callable is being registered
                // as a request handler for the same path+method multiple times.
                let unique_handlers = relevant_handler_ids
                    .iter()
                    .unique_by(|id| db[**id].raw_identifiers_id())
                    .collect::<Vec<_>>();
                if unique_handlers.len() > 1 {
                    push_router_conflict_diagnostic(
                        path,
                        method,
                        &unique_handlers,
                        db,
                        package_graph,
                        diagnostics,
                    );
                }
            }
        }

        if n_diagnostics == diagnostics.len() {
            Ok(())
        } else {
            Err(())
        }
    }

    /// Make sure that the user-registered paths don't conflict with each other.
    /// In other words: we won't encounter any issue when creating this router.
    ///
    /// How do we do that?
    ///
    /// By trying to create the router in the compiler itself!
    /// If it works now, it'll work at runtime too.
    fn detect_path_conflicts(
        db: &RawUserComponentDb,
        component_ids: &[UserComponentId],
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Result<matchit::Router<()>, ()> {
        let mut path_router = matchit::Router::new();
        let mut errored = false;
        for id in component_ids.iter() {
            let UserComponent::RequestHandler { router_key, .. } = &db[id] else {
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
                _ => {
                    errored = true;
                    push_matchit_diagnostic(
                        db,
                        &router_key.path,
                        *id,
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

    /// Determine, for each request handler, which fallback should be used if an incoming request
    /// doesn't match any of the user-registered routes.
    ///
    /// There are two kinds of "misses":
    /// 1. there is a registered route that matches the incoming request path, but the method doesn't match
    ///    any of the methods registered for that route.
    /// 2. there is no registered route that matches the incoming request path.
    ///
    /// This method only looks at the 2nd case and returns a mapping from request handlers to fallbacks.
    fn assign_fallbacks(
        mut validation_router: matchit::Router<()>,
        scope_based_fallback_router: &ScopeBasedFallbackTree,
        component_ids: &[UserComponentId],
        db: &RawUserComponentDb,
        scope_graph: &ScopeGraph,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Result<
        (
            BTreeMap<UserComponentId, UserComponentId>,
            BTreeMap<String, UserComponentId>,
        ),
        (),
    > {
        let n_diagnostics = diagnostics.len();

        let mut path_based_fallback_router = matchit::Router::new();
        let mut path_catchall2fallback_id = BTreeMap::new();
        for id in component_ids.iter() {
            let UserComponent::Fallback { .. } = &db[id] else {
                continue;
            };
            let path_prefix = &db.fallback_id2path_prefix[id];
            // If there is a nested blueprint with a path prefix, we register a path-based fallback
            // for all incoming requests that match that prefix.
            let Some(path_prefix) = path_prefix else {
                continue;
            };
            let parsed_prefix = RoutePath::parse(path_prefix.to_owned());

            let fallback_path = {
                let mut fallback_path = None;
                if let Some(details) = parsed_prefix.parameters.values().last() {
                    let n_chars = parsed_prefix.raw.chars().count();
                    if n_chars - 1 == details.end {
                        // The last params is at the end of the path
                        if details.catch_all {
                            // No need to register a path-based fallback if we have a trailing catch-all
                            continue;
                        } else {
                            // We strip the last parameter from the path prefix and substitute it with a catch-all
                            // to create a fallback path.
                            let stripped: String = parsed_prefix
                                .raw
                                .chars()
                                .dropping_back(details.end - details.start)
                                .collect();
                            fallback_path = Some(format!("{stripped}{{*catch_all}}"));
                        }
                    }
                };
                fallback_path.unwrap_or_else(|| format!("{}{{*catch_all}}", parsed_prefix.raw))
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

            path_catchall2fallback_id.insert(fallback_path.clone(), *id);
            path_based_fallback_router
                .insert(fallback_path, *id)
                .unwrap();
        }

        let mut handler_id2fallback_id = BTreeMap::new();
        // We now iterate over all request handlers to verify that path-based and scope-based
        // fallbacks match.
        // If they don't, we emit a diagnostic: there is ambiguity in the routing logic and we
        // don't know which fallback to use.
        for id in component_ids.iter() {
            let UserComponent::RequestHandler {
                router_key,
                scope_id,
                ..
            } = &db[id]
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
                    handler_id2fallback_id.insert(*id, scope_fallback_id);
                }
                Some(path_fallback_id) => {
                    if path_fallback_id != scope_fallback_id {
                        let path_fallback_scope_id = {
                            *db[path_fallback_id]
                                .scope_id()
                                .direct_parent_ids(scope_graph)
                                .iter()
                                .next()
                                .unwrap()
                        };
                        let scope_fallback_scope_id = {
                            *db[scope_fallback_id]
                                .scope_id()
                                .direct_parent_ids(scope_graph)
                                .iter()
                                .next()
                                .unwrap()
                        };
                        if scope_fallback_scope_id
                            .is_descendant_of(path_fallback_scope_id, scope_graph)
                        {
                            // We are looking at a situation like the following:
                            //
                            // bp.nest_at("/path_prefix", {
                            //    bp.fallback(f!(...));
                            //    bp.nest({
                            //        bp.route(GET, "/yo", f!(...));
                            //        bp.fallback(f!(...));
                            //    });
                            // });
                            //
                            // And this is fine, since the scope-based fallback is obviously the
                            // desired one since it wraps closer to the route.
                            handler_id2fallback_id.insert(*id, scope_fallback_id);
                            continue;
                        }

                        push_fallback_ambiguity_diagnostic(
                            db,
                            scope_fallback_id,
                            path_fallback_id,
                            *id,
                            package_graph,
                            diagnostics,
                        );
                    } else {
                        // Good: they both use the same fallback.
                        handler_id2fallback_id.insert(*id, path_fallback_id);
                    }
                }
            }
        }

        if n_diagnostics == diagnostics.len() {
            Ok((handler_id2fallback_id, path_catchall2fallback_id))
        } else {
            Err(())
        }
    }

    /// There are two kinds of routing "misses":
    /// 1. there is a registered route that matches the incoming request path, but the method doesn't match
    ///    any of the methods registered for that route.
    /// 2. there is no registered route that matches the incoming request path.
    ///
    /// This method checks the first case: do all registered routes for a certain path
    /// expect the same fallback when the method doesn't match?
    fn check_method_not_allowed_fallbacks(
        route_id2fallback_id: &BTreeMap<UserComponentId, UserComponentId>,
        component_ids: &[UserComponentId],
        db: &RawUserComponentDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Result<(), ()> {
        let n_diagnostics = diagnostics.len();

        let mut method_aware_router = matchit::Router::<u32>::new();
        // Route id <> (fallback_id <> (handler_id <> method guards))
        let mut map: BTreeMap<
            u32,
            BTreeMap<UserComponentId, BTreeMap<UserComponentId, BTreeSet<String>>>,
        > = BTreeMap::default();
        let mut next_route_id = 0;
        for id in component_ids.iter() {
            let UserComponent::RequestHandler { router_key, .. } = &db[id] else {
                continue;
            };
            let method_guard = match &router_key.method_guard {
                MethodGuard::Any { .. } => {
                    // `None` stands for the `ANY` guard, it matches all methods
                    // and we have already checked that we don't have any overlap when it comes
                    // to method routing, so we can safely ignore it since we won't have any
                    // other entry for this path.
                    continue;
                }
                MethodGuard::Some(g) => g,
            };
            let route_id = match method_aware_router.at_mut(router_key.path.as_str()) {
                Ok(match_) => *match_.value,
                Err(_) => {
                    let route_id = next_route_id;
                    next_route_id += 1;
                    method_aware_router
                        .insert(router_key.path.clone(), route_id)
                        .unwrap();
                    route_id
                }
            };
            let fallback_id = route_id2fallback_id[id];
            map.entry(route_id)
                .or_default()
                .entry(fallback_id)
                .or_default()
                .insert(*id, method_guard.clone());
        }
        for fallback_id2handler_id in map.values() {
            if fallback_id2handler_id.len() == 1 {
                // Good: there is only one fallback for all handlers registered against this route.
                continue;
            }

            let methods_without_handler = {
                let mut set: BTreeSet<String> =
                    METHODS.into_iter().map(ToOwned::to_owned).collect();
                for handler_id2methods in fallback_id2handler_id.values() {
                    for methods in handler_id2methods.values() {
                        for method in methods {
                            set.remove(method);
                        }
                    }
                }
                set
            };
            push_fallback_method_ambiguity_diagnostic(
                methods_without_handler,
                fallback_id2handler_id,
                db,
                package_graph,
                diagnostics,
            );
        }

        if n_diagnostics == diagnostics.len() {
            Ok(())
        } else {
            Err(())
        }
    }
}

/// A tree that contains a node for each registered fallback (as well as the default one, if needed).
///
/// The tree is built by traversing the scope graph and for each scope that has a fallback, we
/// register a node in the tree.
/// A node is a child of another node if the scope it represents is a descendant of the scope of
/// the parent node.
#[derive(Debug)]
struct ScopeBasedFallbackTree {
    nodes: Vec<FallbackNode>,
}

impl ScopeBasedFallbackTree {
    fn new(
        scope_id2fallback_id: &BiHashMap<ScopeId, UserComponentId>,
        scope_graph: &ScopeGraph,
    ) -> Self {
        let root = FallbackNode {
            scope_id: scope_graph.root_scope_id(),
            fallback_id: *scope_id2fallback_id
                .get_by_left(&scope_graph.root_scope_id())
                .unwrap(),
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
                        fallback_id: *fallback_id,
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
    ) -> UserComponentId {
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
            break 'outer;
        }
        current.fallback_id
    }
}

#[derive(Debug)]
struct FallbackNode {
    scope_id: ScopeId,
    fallback_id: UserComponentId,
    children_ids: Vec<usize>,
}

static METHODS: [&str; 9] = [
    "GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS", "CONNECT", "TRACE",
];

fn push_fallback_ambiguity_diagnostic(
    raw_user_component_db: &RawUserComponentDb,
    scope_fallback_id: UserComponentId,
    path_fallback_id: UserComponentId,
    route_id: UserComponentId,
    package_graph: &PackageGraph,
    diagnostics: &mut Vec<miette::Error>,
) {
    let UserComponent::RequestHandler { router_key, .. } = &raw_user_component_db[route_id] else {
        unreachable!()
    };
    let route_location = raw_user_component_db.get_location(route_id);
    let route_source = try_source!(route_location, package_graph, diagnostics);
    let label = route_source
        .as_ref()
        .map(|source| {
            diagnostic::get_route_path_span(&source, route_location)
                .labeled("The route was registered here".to_string())
        })
        .flatten();
    let route_repr = router_key.diagnostic_repr();
    let scope_fallback = {
        let UserComponent::Fallback {
            raw_callable_identifiers_id,
            ..
        } = &raw_user_component_db[scope_fallback_id]
        else {
            unreachable!()
        };
        format!(
            "`{}`",
            raw_user_component_db.identifiers_interner[*raw_callable_identifiers_id]
                .fully_qualified_path()
                .join("::")
        )
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
    let diagnostic = CompilerDiagnostic::builder(error)
        .optional_source(route_source)
        .optional_label(label)
        .help(format!(
            "You can fix this by registering `{route_repr}` against the nested blueprint \
            with `{path_prefix}` as prefix. All `{path_prefix}`-prefixed routes would then \
            be using `{path_fallback}` as fallback."
        ));
    diagnostics.push(diagnostic.build().into());
}

fn push_fallback_method_ambiguity_diagnostic(
    methods_without_handler: BTreeSet<String>,
    fallback_id2handler_id: &BTreeMap<UserComponentId, BTreeMap<UserComponentId, BTreeSet<String>>>,
    raw_user_component_db: &RawUserComponentDb,
    package_graph: &PackageGraph,
    diagnostics: &mut Vec<miette::Error>,
) {
    use std::fmt::Write;

    let request_handler_id = *fallback_id2handler_id
        .values()
        .next()
        .unwrap()
        .keys()
        .next()
        .unwrap();
    let UserComponent::RequestHandler { router_key, .. } =
        &raw_user_component_db[request_handler_id]
    else {
        unreachable!()
    };
    let route_path = router_key.path.as_str();
    let mut err_msg = "Routing logic can't be ambiguous.\n\
        You registered:\n"
        .to_string();
    let mut first_snippet: Option<AnnotatedSnippet> = None;
    let mut annotated_snippets = Vec::with_capacity(fallback_id2handler_id.len());
    for (i, (fallback_id, handler_id2methods)) in fallback_id2handler_id.iter().enumerate() {
        let fallback_path = {
            let UserComponent::Fallback {
                raw_callable_identifiers_id,
                ..
            } = &raw_user_component_db[*fallback_id]
            else {
                unreachable!()
            };
            let location = raw_user_component_db.get_location(*fallback_id);
            if let Some(source) = try_source!(location, package_graph, diagnostics) {
                let label = diagnostic::get_f_macro_invocation_span(&source, location)
                    .labeled(format!("The {} fallback", ZeroBasedOrdinal::from(i)));
                let snippet = AnnotatedSnippet::new_optional(source, label);
                if first_snippet.is_none() {
                    first_snippet = Some(snippet);
                } else {
                    annotated_snippets.push(snippet);
                }
            }
            let fallback_path = raw_user_component_db.identifiers_interner
                [*raw_callable_identifiers_id]
                .fully_qualified_path()
                .join("::");
            format!("`{fallback_path}`")
        };

        let handler_methods: Vec<_> = handler_id2methods.values().flat_map(|s| s.iter()).collect();
        write!(
            &mut err_msg,
            "- {fallback_path} as the fallback handler for your",
        )
        .unwrap();
        if handler_methods.len() == 1 {
            let handler_method = handler_methods[0];
            writeln!(&mut err_msg, " `{handler_method} {route_path}` route.",).unwrap();
        } else {
            let handler_methods = {
                let mut buffer = String::new();
                comma_separated_list(
                    &mut buffer,
                    handler_methods.into_iter(),
                    ToOwned::to_owned,
                    "or",
                )
                .unwrap();
                buffer
            };
            writeln!(&mut err_msg, " {handler_methods} `{route_path}` routes.",).unwrap();
        }
    }

    let methods_without_handlers = if !methods_without_handler.is_empty() {
        let mut buffer = " (".to_string();
        comma_separated_list(
            &mut buffer,
            methods_without_handler.iter(),
            ToOwned::to_owned,
            "or",
        )
        .unwrap();
        write!(buffer, ")").unwrap();
        buffer
    } else {
        String::new()
    };
    writeln!(
        &mut err_msg,
        "\nI don't know which fallback handler to invoke for incoming `{route_path}` requests \
             that use a different HTTP method{methods_without_handlers}!"
    )
    .unwrap();
    let error = anyhow::anyhow!(err_msg);
    let mut builder = CompilerDiagnostic::builder(error);
    if let Some(first_snippet) = first_snippet {
        builder = builder
            .source(first_snippet.source_code)
            .labels(first_snippet.labels.into_iter())
            .additional_annotated_snippets(annotated_snippets.into_iter());
    }
    builder = builder.help(format!(
        "Adjust your blueprint to have the same fallback handler for all `{route_path}` routes."
    ));
    diagnostics.push(builder.build().into());
}

fn push_matchit_diagnostic(
    raw_user_component_db: &RawUserComponentDb,
    path: &str,
    raw_user_component_id: UserComponentId,
    error: matchit::InsertError,
    package_graph: &PackageGraph,
    diagnostics: &mut Vec<miette::Error>,
) {
    // We want to control the error message for style consistency with the rest of the
    // diagnostics we emit.
    let error = match error {
        InsertError::Conflict { with } => {
            anyhow!("This route path, `{}`, conflicts with the path of another route you already registered, `{}`.", path, with)
        }
        InsertError::InvalidParam => {
            anyhow!("You can only use path parameters in the form of `{{name}}` or `{{*name}}`. You can use `{{{{` and `}}}}` if you need to escape curly braces.")
        }
        InsertError::InvalidParamSegment => {
            anyhow!("You can only register one path parameter for each path segment.")
        }
        InsertError::InvalidCatchAll => {
            anyhow!("You can only use catch-all parameters at the end of a route path.")
        }
        _ => error.into(),
    };

    let location = raw_user_component_db.get_location(raw_user_component_id);
    let source = try_source!(location, package_graph, diagnostics);
    let label = source
        .as_ref()
        .map(|source| {
            diagnostic::get_route_path_span(&source, location)
                .labeled("The problematic path".to_string())
        })
        .flatten();
    let diagnostic = CompilerDiagnostic::builder(error)
        .optional_source(source)
        .optional_label(label);
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
        let Some(source) = try_source!(location, package_graph, diagnostics) else {
            continue;
        };
        if let Some(s) = diagnostic::get_f_macro_invocation_span(&source, location) {
            let label = s.labeled(format!("The {} conflicting handler", ZeroBasedOrdinal(i)));
            annotated_snippets.push(AnnotatedSnippet::new(source, label));
        }
    }
    let mut annotated_snippets = annotated_snippets.into_iter();
    let mut builder = CompilerDiagnostic::builder(anyhow!(
            "I don't know how to route incoming `{method} {path}` requests: you have registered {n_unique_handlers} \
            different request handlers for this path+method combination."
        ));
    if let Some(first) = annotated_snippets.next() {
        builder = builder
            .source(first.source_code)
            .labels(first.labels.into_iter());
    }
    builder = builder
        .additional_annotated_snippets(annotated_snippets)
        .help(
            "You can only register one request handler for each path+method combination. \
            Remove all but one of the conflicting request handlers."
                .into(),
        );
    diagnostics.push(builder.build().into());
}
