use std::collections::{BTreeMap, BTreeSet};

use ahash::HashMap;

use super::domain::DomainGuard;
use crate::compiler::analyses::components::ComponentId;
use crate::compiler::analyses::user_components::UserComponentId;

/// A mechanism to route incoming requests to the correct handler.
#[derive(Debug)]
pub(crate) enum Router {
    DomainAgnostic(PathRouter),
    DomainBased(DomainRouter),
}

/// A mapping from a handler/fallback id to diagnostic information about the route
/// it serves.
pub(crate) struct RouteInfos(BTreeMap<ComponentId, RouteInfo>);

impl std::ops::Index<ComponentId> for RouteInfos {
    type Output = RouteInfo;

    fn index(&self, id: ComponentId) -> &Self::Output {
        &self.0[&id]
    }
}

/// Route requests to handlers based on their path and HTTP method.
#[derive(Debug)]
pub(crate) struct PathRouter {
    /// A map from the path to the HTTP methods that it can handle.
    pub(crate) path2method_router: BTreeMap<String, LeafRouter>,
    /// The fallback to use if no route matches the incoming request.
    pub(crate) root_fallback_id: ComponentId,
}

/// Route requests to handlers based on their domain, path, and HTTP method.
#[derive(Debug)]
pub(crate) struct DomainRouter {
    /// A map from the domain to the path router for that domain.
    pub(crate) domain2path_router: BTreeMap<DomainGuard, PathRouter>,
    /// The fallback to use if the domain of the incoming request doesn't match any of the domains
    /// we are looking for.
    pub(crate) root_fallback_id: ComponentId,
}

#[derive(Debug)]
pub(crate) struct RouteInfo {
    pub(crate) methods: BTreeSet<String>,
    pub(crate) path: String,
    pub(crate) domain: Option<DomainGuard>,
}

impl std::fmt::Display for RouteInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let domain = self
            .domain
            .as_ref()
            .map(|d| format!(" [for {}]", d))
            .unwrap_or_else(|| String::from(""));

        let methods = if self.methods.is_empty() {
            "*".to_string()
        } else {
            use itertools::Itertools as _;
            self.methods.iter().map(|m| m.as_str()).join(" | ")
        };
        write!(f, "{methods} {}{domain}", self.path)
    }
}

/// A router to dispatch a request to a handler based on its method, after having matched its path.
#[derive(Debug, Clone)]
pub(crate) struct LeafRouter {
    // TODO: we could use a more memory efficient representation here (e.g. a bitset) to describe
    //     the set of methods that a handler can handle.
    pub(crate) handler_id2methods: BTreeMap<ComponentId, BTreeSet<String>>,
    pub(crate) fallback_id: ComponentId,
}

impl LeafRouter {
    /// Return the set of [`ComponentId`]s that can handle the given route, including the fallback.
    pub(crate) fn handler_ids(&self) -> impl Iterator<Item = &ComponentId> {
        self.handler_id2methods
            .keys()
            .chain(std::iter::once(&self.fallback_id))
    }
}

impl Router {
    /// Lift the router we built in a previous pass by replacing all
    /// the low-level [`UserComponentId`]s with high-level [`ComponentId`]s.
    pub(crate) fn lift(
        router: super::user_components::Router,
        user_component_id2component_id: &HashMap<UserComponentId, ComponentId>,
    ) -> Self {
        match router {
            super::user_components::Router::DomainAgnostic(router) => {
                let router = PathRouter::lift(router, user_component_id2component_id);
                Router::DomainAgnostic(router)
            }
            super::user_components::Router::DomainBased(router) => {
                let router = DomainRouter::lift(router, user_component_id2component_id);
                Router::DomainBased(router)
            }
        }
    }

    /// Return the **ordered** set of [`ComponentIds`] that can handle requests, including the fallback.
    ///
    /// It returns a set, rather than a vector, to avoid duplicates (e.g. the same fallback may appear
    /// multiple times in a domain-based router).
    /// It returns an **ordered** set to guarantee a deterministic ordering that can be used to assign
    /// unique and stable ids to the handlers.
    pub(crate) fn handler_ids(&self) -> BTreeSet<ComponentId> {
        match self {
            Router::DomainAgnostic(router) => router
                .handler_ids()
                .chain(std::iter::once(&router.root_fallback_id))
                .cloned()
                .collect(),
            Router::DomainBased(router) => router
                .domain2path_router
                .values()
                .flat_map(|path_router| path_router.handler_ids())
                .chain(std::iter::once(&router.root_fallback_id))
                .cloned()
                .collect(),
        }
    }

    pub(crate) fn route_infos(&self) -> RouteInfos {
        fn _route_infos(
            router: &PathRouter,
            domain_guard: Option<DomainGuard>,
            handler_id2route_info: &mut BTreeMap<ComponentId, RouteInfo>,
        ) {
            for (path, method_router) in router.path2method_router.iter() {
                for (handler_id, methods) in method_router.handler_id2methods.iter() {
                    let router_info = RouteInfo {
                        methods: methods.to_owned(),
                        path: path.to_owned(),
                        domain: domain_guard.clone(),
                    };
                    let previous = handler_id2route_info.insert(*handler_id, router_info);
                    assert!(
                        previous.is_none(),
                        "Each handler ID is uniquely associated with a route."
                    )
                }
                handler_id2route_info.insert(
                    method_router.fallback_id,
                    RouteInfo {
                        methods: Default::default(),
                        path: path.to_owned(),
                        domain: domain_guard.clone(),
                    },
                );
            }
            handler_id2route_info.insert(
                router.root_fallback_id,
                RouteInfo {
                    methods: Default::default(),
                    path: "*".into(),
                    domain: domain_guard.clone(),
                },
            );
        }

        let mut handler_id2route_info = BTreeMap::new();
        match self {
            Router::DomainAgnostic(router) => {
                _route_infos(router, None, &mut handler_id2route_info);
            }
            Router::DomainBased(router) => {
                for (domain_guard, path_router) in router.domain2path_router.iter() {
                    _route_infos(
                        path_router,
                        Some(domain_guard.clone()),
                        &mut handler_id2route_info,
                    );
                }
                handler_id2route_info.insert(
                    router.root_fallback_id,
                    RouteInfo {
                        methods: Default::default(),
                        path: "*".into(),
                        domain: None,
                    },
                );
            }
        }
        RouteInfos(handler_id2route_info)
    }
}

impl DomainRouter {
    /// Lift the router we built in a previous pass by replacing all
    /// the low-level [`UserComponentId`]s with high-level [`ComponentId`]s.
    pub(crate) fn lift(
        router: super::user_components::DomainRouter,
        user_component_id2component_id: &HashMap<UserComponentId, ComponentId>,
    ) -> Self {
        let domain2path_router = router
            .domain2path_router
            .into_iter()
            .map(|(domain_guard, path_router)| {
                let path_router = PathRouter::lift(path_router, user_component_id2component_id);
                (domain_guard, path_router)
            })
            .collect();
        let root_fallback_id = user_component_id2component_id[&router.root_fallback_id];
        DomainRouter {
            domain2path_router,
            root_fallback_id,
        }
    }
}

impl PathRouter {
    /// Lift the router we built in a previous pass by replacing all
    /// the low-level [`UserComponentId`]s with high-level [`ComponentId`]s.
    pub(crate) fn lift(
        router: super::user_components::PathRouter,
        user_component_id2component_id: &HashMap<UserComponentId, ComponentId>,
    ) -> Self {
        let path2method_router = router
            .path2method_router
            .into_iter()
            .map(|(route_path, leaf_router)| {
                let handler_id2methods = leaf_router
                    .handler_id2methods
                    .into_iter()
                    .filter_map(|(user_component_id, methods)| {
                        user_component_id2component_id
                            .get(&user_component_id)
                            .map(|&component_id| (component_id, methods))
                    })
                    .collect();
                let fallback_id = user_component_id2component_id[&leaf_router.fallback_id];
                (
                    route_path,
                    LeafRouter {
                        handler_id2methods,
                        fallback_id,
                    },
                )
            })
            .collect();
        let root_fallback_id = user_component_id2component_id[&router.root_fallback_id];
        Self {
            path2method_router,
            root_fallback_id,
        }
    }

    pub(crate) fn handler_ids(&self) -> impl Iterator<Item = &ComponentId> {
        self.path2method_router
            .values()
            .flat_map(|leaf_router| leaf_router.handler_ids())
            .chain(std::iter::once(&self.root_fallback_id))
    }
}
