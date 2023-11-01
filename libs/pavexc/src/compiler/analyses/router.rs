use std::collections::{BTreeMap, BTreeSet};

use ahash::HashMap;

use crate::compiler::analyses::components::ComponentId;
use crate::compiler::analyses::user_components::UserComponentId;

#[derive(Debug)]
pub(crate) struct Router {
    pub(crate) route_path2sub_router: BTreeMap<String, LeafRouter>,
    /// The fallback to use if no route matches the incoming request.
    pub(crate) root_fallback_id: ComponentId,
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
        let route_path2sub_router = router
            .route_path2sub_router
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
            route_path2sub_router,
            root_fallback_id,
        }
    }
}
