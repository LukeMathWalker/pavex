use std::collections::BTreeMap;

use crate::{
    compiler::{analyses::domain::DomainGuard, component::DefaultStrategy, interner::Interner},
    diagnostic::Registration,
    rustdoc::GlobalItemId,
};
use ahash::HashMap;
use indexmap::IndexMap;
use pavex_bp_schema::{
    CloningStrategy, Import, Lifecycle, Lint, LintSetting, Location, RawIdentifiers,
};

use super::{ScopeId, UserComponent, UserComponentId};

/// Data that we need to keep track of as we collect and process all user-registered components.
///
/// Some of these data structures will be kept around for later compilation passes;
/// others will be discarded after this compilation pass.
#[derive(Default)]
pub(super) struct AuxiliaryData {
    pub(super) component_interner: la_arena::Arena<UserComponent>,
    /// Associate each component with the scope it belongs to.
    ///
    /// Invariants: there is an entry for every single user component.
    pub(super) id2scope_id: la_arena::ArenaMap<UserComponentId, ScopeId>,
    /// A list of imports to be resolved.
    ///
    /// For each import, we keep track of the scope that it was declared in.
    /// That scope will be used as the scope for the imported components.
    pub(super) imports: Vec<(Import, ScopeId)>,
    pub(super) annotation_interner: Interner<GlobalItemId>,
    pub(super) identifiers_interner: Interner<RawIdentifiers>,
    /// Associate each user-registered component with the location it was
    /// registered at.
    ///
    /// Invariants: there is an entry for every single user component.
    pub(super) id2registration: HashMap<UserComponentId, Registration>,
    /// Associate each user-registered component with its lifecycle.
    ///
    /// Invariants: there is an entry for every single user component.
    pub(super) id2lifecycle: HashMap<UserComponentId, Lifecycle>,
    /// Associate each user-registered component with its lint overrides, if any.
    /// If there is no entry for a component, there are no overrides.
    pub(super) id2lints: HashMap<UserComponentId, BTreeMap<Lint, LintSetting>>,
    /// Determine if a type can be cloned or not.
    ///
    /// Invariants: there is an entry for every constructor, configuration type and prebuilt type.
    pub(super) id2cloning_strategy: HashMap<UserComponentId, CloningStrategy>,
    /// Determine if a configuration type should have a default.
    ///
    /// Invariants: there is an entry for configuration type.
    pub(super) config_id2default_strategy: HashMap<UserComponentId, DefaultStrategy>,
    /// Associate each request handler with the ordered list of middlewares that wrap around it.
    ///
    /// Invariants: there is an entry for every single request handler.
    pub(super) handler_id2middleware_ids: HashMap<UserComponentId, Vec<UserComponentId>>,
    /// Associate each request handler with the ordered list of error observers
    /// that must be invoked if there is an error while handling a request.
    ///
    /// Invariants: there is an entry for every single request handler.
    pub(super) handler_id2error_observer_ids: HashMap<UserComponentId, Vec<UserComponentId>>,
    /// Associate each user-registered fallback with the path prefix of the `Blueprint`
    /// it was registered against.
    /// If it was registered against a deeply nested `Blueprint`, it contains the **concatenated**
    /// path prefixes of all the `Blueprint`s that it was nested under.
    ///
    /// Invariants: there is an entry for every single fallback.
    pub(super) fallback_id2path_prefix: HashMap<UserComponentId, Option<String>>,
    /// Associate each user-registered fallback with the domain guard of the `Blueprint`
    /// it was registered against, if any.
    /// If it was registered against a deeply nested `Blueprint`, it contains the domain guard
    /// of the **innermost** `Blueprint` with a non-empty domain guard that it was nested under.
    ///
    /// Invariants: there is an entry for every single fallback.
    pub(super) fallback_id2domain_guard: HashMap<UserComponentId, Option<DomainGuard>>,
    /// Associate each domain guard with the location it was registered at against the `Blueprint`.
    ///
    /// The same guard can be registered at multiple locations, so we use a `Vec` to store them.
    pub(super) domain_guard2locations: IndexMap<DomainGuard, Vec<Location>>,
}

impl AuxiliaryData {
    /// A helper function to intern a component without forgetting to do the necessary
    /// bookeeping for the metadata (location and lifecycle) that are common to all
    /// components.
    pub(super) fn intern_component(
        &mut self,
        component: UserComponent,
        scope_id: ScopeId,
        lifecycle: Lifecycle,
        registration: Registration,
    ) -> UserComponentId {
        let component_id = self.component_interner.alloc(component);
        self.id2lifecycle.insert(component_id, lifecycle);
        self.id2registration.insert(component_id, registration);
        self.id2scope_id.insert(component_id, scope_id);
        component_id
    }

    /// Iterate over all user components (and their ids) discovered up to this point.
    pub(super) fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (UserComponentId, &UserComponent)> {
        self.component_interner.iter()
    }

    /// Iterate over all user components discovered up to this point.
    pub(super) fn components(&self) -> impl ExactSizeIterator<Item = &UserComponent> {
        self.component_interner.values()
    }

    /// Validate that all internal invariants are satisfied.
    #[cfg(debug_assertions)]
    pub(super) fn check_invariants(&self) {
        use UserComponent::*;

        for (id, component) in self.component_interner.iter() {
            assert!(
                self.id2lifecycle.contains_key(&id),
                "There is no lifecycle registered for the user-provided {} #{id:?}",
                component.kind()
            );
            assert!(
                self.id2registration.contains_key(&id),
                "There is no location registered for the user-provided {} #{id:?}",
                component.kind()
            );
            assert!(
                self.id2scope_id.contains_idx(id),
                "There is no scope registered for the user-provided {} #{id:?}",
                component.kind()
            );
            match component {
                Constructor { .. } | PrebuiltType { .. } => {
                    assert!(
                        self.id2cloning_strategy.contains_key(&id),
                        "There is no cloning strategy registered for the user-registered {} #{id:?}",
                        component.kind(),
                    );
                }
                ConfigType { .. } => {
                    assert!(
                        self.id2cloning_strategy.contains_key(&id),
                        "There is no cloning strategy registered for the user-registered {} #{id:?}",
                        component.kind(),
                    );
                    assert!(
                        self.config_id2default_strategy.contains_key(&id),
                        "There is no default strategy registered for the user-registered {} #{id:?}",
                        component.kind(),
                    );
                }
                RequestHandler { .. } => {
                    assert!(
                        self.handler_id2middleware_ids.contains_key(&id),
                        "The middleware chain is missing for the user-registered request handler #{:?}",
                        id
                    );
                    assert!(
                        self.handler_id2error_observer_ids.contains_key(&id),
                        "The list of error observers is missing for the user-registered request handler #{:?}",
                        id
                    );
                }
                Fallback { .. } => {
                    assert!(
                        self.handler_id2middleware_ids.contains_key(&id),
                        "The middleware chain is missing for the user-registered fallback #{:?}",
                        id
                    );
                    assert!(
                        self.handler_id2error_observer_ids.contains_key(&id),
                        "The list of error observers is missing for the user-registered fallback #{:?}",
                        id
                    );
                    assert!(
                        self.fallback_id2path_prefix.contains_key(&id),
                        "There is no path prefix associated with the user-registered fallback #{:?}",
                        id
                    );
                    assert!(
                        self.fallback_id2domain_guard.contains_key(&id),
                        "There is no domain guard associated with the user-registered fallback #{:?}",
                        id
                    );
                }
                ErrorHandler { .. }
                | WrappingMiddleware { .. }
                | PostProcessingMiddleware { .. }
                | PreProcessingMiddleware { .. }
                | ErrorObserver { .. } => {}
            }
        }
    }
}

impl std::ops::Index<UserComponentId> for AuxiliaryData {
    type Output = UserComponent;

    fn index(&self, index: UserComponentId) -> &Self::Output {
        &self.component_interner[index]
    }
}

impl std::ops::Index<&UserComponentId> for AuxiliaryData {
    type Output = UserComponent;

    fn index(&self, index: &UserComponentId) -> &Self::Output {
        &self.component_interner[*index]
    }
}
