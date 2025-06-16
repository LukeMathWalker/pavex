use super::UserComponentId;
use super::{ScopeGraph, ScopeId};
use super::{UserComponent, auxiliary::AuxiliaryData};
use super::{blueprint::process_blueprint, router::Router};
use crate::compiler::analyses::user_components::annotations::{
    register_imported_components, resolve_annotation_coordinates,
};
use crate::compiler::analyses::user_components::imports::resolve_imports;
use crate::compiler::analyses::user_components::paths::FQPaths;
use crate::compiler::component::ConfigType;
use crate::compiler::{
    analyses::{computations::ComputationDb, prebuilt_types::PrebuiltTypeDb},
    component::DefaultStrategy,
};
use crate::diagnostic::TargetSpan;
use crate::diagnostic::{DiagnosticSink, Registration};
use crate::{language::FQPath, rustdoc::CrateCollection};
use ahash::HashMap;
use guppy::PackageId;
use indexmap::IndexSet;
use pavex_bp_schema::{Blueprint, CloningStrategy, Lifecycle, Lint, LintSetting};
use pavex_cli_diagnostic::AnyhowBridge;
use std::collections::BTreeMap;

/// A database that contains all the user components that have been registered against the
/// `Blueprint` for the application.
///
/// For each component, we keep track of:
/// - the source code location where it was registered (for error reporting purposes);
/// - the lifecycle of the component;
/// - the scope that the component belongs to.
///
/// Some basic validation has been carried out:
/// - the callable associated to each component has been resolved and added to the
///   provided [`ComputationDb`].
/// - there are no conflicting routes.
#[derive(Debug)]
pub struct UserComponentDb {
    component_interner: la_arena::Arena<UserComponent>,
    /// Associates each user-registered component with the scope it belongs to.
    ///
    /// Invariants: there is an entry for every single user component.
    id2scope_id: la_arena::ArenaMap<UserComponentId, ScopeId>,
    /// Associate each user-registered component with the location it was
    /// registered at.
    ///
    /// Invariants: there is an entry for every single user component.
    id2registration: la_arena::ArenaMap<UserComponentId, Registration>,
    /// Associate each user-registered component with its lifecycle.
    ///
    /// Invariants: there is an entry for every single user component.
    id2lifecycle: la_arena::ArenaMap<UserComponentId, Lifecycle>,
    /// Associate each user-registered component with its lint overrides, if any.
    /// If there is no entry for a component, there are no overrides.
    id2lints: HashMap<UserComponentId, BTreeMap<Lint, LintSetting>>,
    /// For each constructible component, determine if it can be cloned or not.
    ///
    /// Invariants: there is an entry for every constructor and prebuilt type.
    id2cloning_strategy: HashMap<UserComponentId, CloningStrategy>,
    /// Assign the id of each config to its type and key.
    ///
    /// Invariants: there is an entry for every config type.
    config_id2type: HashMap<UserComponentId, ConfigType>,
    /// Determine if a configuration type should have a default.
    ///
    /// Invariants: there is an entry for configuration type.
    config_id2default_strategy: HashMap<UserComponentId, DefaultStrategy>,
    /// Determine if a configuration type should be included in the generated `ApplicationConfig`
    /// even if unused.
    ///
    /// Invariants: there is an entry for configuration type.
    config_id2include_if_unused: HashMap<UserComponentId, bool>,
    /// Associate each request handler with the ordered list of middlewares that wrap around it.
    ///
    /// Invariants: there is an entry for every single request handler.
    handler_id2middleware_ids: HashMap<UserComponentId, Vec<UserComponentId>>,
    /// Associate each request handler with the ordered list of error observers
    /// that must be invoked when an error occurs while handling a request.
    ///
    /// Invariants: there is an entry for every single request handler.
    handler_id2error_observer_ids: HashMap<UserComponentId, Vec<UserComponentId>>,
    scope_graph: ScopeGraph,
}

impl UserComponentDb {
    /// Process a `Blueprint` and return a `UserComponentDb` that contains all the user components
    /// that have been registered against it.
    ///
    /// The callable associated to each component will be resolved and added to the
    /// provided [`ComputationDb`].
    #[tracing::instrument(name = "Build user component database", skip_all)]
    pub(crate) fn build(
        bp: &Blueprint,
        computation_db: &mut ComputationDb,
        prebuilt_type_db: &mut PrebuiltTypeDb,
        krate_collection: &CrateCollection,
        diagnostics: &DiagnosticSink,
    ) -> Result<(Router, Self), ()> {
        /// Exit early if there is at least one error.
        macro_rules! exit_on_errors {
            ($var:ident) => {
                if !$var.is_empty() {
                    return Err(());
                }
            };
        }

        let mut aux = AuxiliaryData::default();
        let mut scope_graph_builder = process_blueprint(bp, &mut aux, diagnostics);
        let mut paths = FQPaths::new();
        paths.process_identifiers(&aux, krate_collection.package_graph(), diagnostics);
        let imported_modules = resolve_imports(&aux, krate_collection.package_graph(), diagnostics);

        precompute_crate_docs(
            krate_collection,
            paths.values(),
            imported_modules.iter().map(|(i, _)| &i.package_id),
            diagnostics,
        );

        register_imported_components(
            &imported_modules,
            &mut aux,
            &mut scope_graph_builder,
            computation_db,
            prebuilt_type_db,
            krate_collection,
            diagnostics,
        );
        resolve_annotation_coordinates(
            &mut aux,
            computation_db,
            prebuilt_type_db,
            krate_collection,
            diagnostics,
        );
        paths.resolve(
            &mut aux,
            computation_db,
            prebuilt_type_db,
            krate_collection,
            diagnostics,
        );
        exit_on_errors!(diagnostics);

        let scope_graph = scope_graph_builder.build();
        let router = Router::new(&aux, computation_db, &scope_graph, diagnostics)?;
        exit_on_errors!(diagnostics);

        #[cfg(debug_assertions)]
        aux.check_invariants();

        let AuxiliaryData {
            component_interner,
            id2scope_id,
            id2registration,
            id2lints,
            id2cloning_strategy,
            id2lifecycle,
            config_id2type,
            config_id2default_strategy,
            config_id2include_if_unused,
            handler_id2middleware_ids,
            handler_id2error_observer_ids,
            annotation_coordinates_interner: _,
            annotation_interner: _,
            fallible_id2error_handler_id: _,
            imports: _,
            identifiers_interner: _,
            fallback_id2domain_guard: _,
            fallback_id2path_prefix: _,
            domain_guard2locations: _,
        } = aux;

        Ok((
            router,
            Self {
                component_interner,
                id2scope_id,
                id2registration,
                id2cloning_strategy,
                id2lifecycle,
                config_id2type,
                config_id2default_strategy,
                config_id2include_if_unused,
                handler_id2middleware_ids,
                handler_id2error_observer_ids,
                scope_graph,
                id2lints,
            },
        ))
    }

    /// Iterate over all the user components in the database, returning their id and the associated
    /// `UserComponent`.
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (UserComponentId, &UserComponent)> + DoubleEndedIterator
    {
        self.component_interner.iter()
    }

    /// Iterate over all the constructor components in the database, returning their id and the
    /// associated `UserComponent`.
    pub fn constructors(
        &self,
    ) -> impl DoubleEndedIterator<Item = (UserComponentId, &UserComponent)> {
        self.component_interner
            .iter()
            .filter(|(_, c)| matches!(c, UserComponent::Constructor { .. }))
    }

    /// Iterate over all the prebuilt types in the database, returning their id and the
    /// associated `UserComponent`.
    pub fn prebuilt_types(
        &self,
    ) -> impl DoubleEndedIterator<Item = (UserComponentId, &UserComponent)> {
        self.component_interner
            .iter()
            .filter(|(_, c)| matches!(c, UserComponent::PrebuiltType { .. }))
    }

    /// Iterate over all the config types in the database, returning their id and the
    /// associated `UserComponent`.
    pub fn config_types(
        &self,
    ) -> impl DoubleEndedIterator<Item = (UserComponentId, &UserComponent)> {
        self.component_interner
            .iter()
            .filter(|(_, c)| matches!(c, UserComponent::ConfigType { .. }))
    }

    /// Iterate over all the request handler components in the database, returning their id and the
    /// associated `UserComponent`.
    ///
    /// It returns both routes (i.e. handlers that are registered against a specific path and method
    /// guard) and fallback handlers.
    pub fn request_handlers(
        &self,
    ) -> impl DoubleEndedIterator<Item = (UserComponentId, &UserComponent)> {
        self.component_interner.iter().filter(|(_, c)| {
            matches!(
                c,
                UserComponent::RequestHandler { .. } | UserComponent::Fallback { .. }
            )
        })
    }

    /// Iterate over all the wrapping middleware components in the database, returning their id and the
    /// associated `UserComponent`.
    pub fn wrapping_middlewares(
        &self,
    ) -> impl DoubleEndedIterator<Item = (UserComponentId, &UserComponent)> {
        self.component_interner
            .iter()
            .filter(|(_, c)| matches!(c, UserComponent::WrappingMiddleware { .. }))
    }

    /// Iterate over all the post-processing middleware components in the database, returning their id and the
    /// associated `UserComponent`.
    pub fn post_processing_middlewares(
        &self,
    ) -> impl DoubleEndedIterator<Item = (UserComponentId, &UserComponent)> {
        self.component_interner
            .iter()
            .filter(|(_, c)| matches!(c, UserComponent::PostProcessingMiddleware { .. }))
    }

    /// Iterate over all the pre-processing middleware components in the database, returning their id and the
    /// associated `UserComponent`.
    pub fn pre_processing_middlewares(
        &self,
    ) -> impl DoubleEndedIterator<Item = (UserComponentId, &UserComponent)> {
        self.component_interner
            .iter()
            .filter(|(_, c)| matches!(c, UserComponent::PreProcessingMiddleware { .. }))
    }

    /// Iterate over all the error observer components in the database, returning their id and the
    /// associated `UserComponent`.
    pub fn error_observers(
        &self,
    ) -> impl DoubleEndedIterator<Item = (UserComponentId, &UserComponent)> {
        self.component_interner
            .iter()
            .filter(|(_, c)| matches!(c, UserComponent::ErrorObserver { .. }))
    }

    /// Return the scope of the component with the given id.
    pub fn scope_id(&self, id: UserComponentId) -> ScopeId {
        self.id2scope_id[id]
    }

    /// Return the lifecycle of the component with the given id.
    pub fn lifecycle(&self, id: UserComponentId) -> Lifecycle {
        self.id2lifecycle[id]
    }

    /// Return the registration metadata for the component with the given id.
    ///
    /// It's the primary entrypoint when you're building a diagnostic that
    /// includes a span from the source file where the component was registered.
    pub fn registration(&self, id: UserComponentId) -> &Registration {
        &self.id2registration[id]
    }

    /// Return the target metadata for the component with the given id.
    ///
    /// It's the primary entrypoint when you're building a diagnostic that
    /// includes a span from the source file where the component was registered.
    pub fn registration_target(&self, id: UserComponentId) -> TargetSpan {
        TargetSpan::Registration(self.registration(id), self[id].kind())
    }

    /// Return the cloning strategy of the component with the given id.
    /// This is going to be `Some(..)` for constructor and prebuilt type components,
    /// and `None` for all other components.
    pub fn cloning_strategy(&self, id: UserComponentId) -> Option<&CloningStrategy> {
        self.id2cloning_strategy.get(&id)
    }

    /// Return the resolved type and key for the configuration component with the given id.
    /// This is going to be `Some(..)` for configuration components,
    /// and `None` for all other components.
    pub fn config_type(&self, id: UserComponentId) -> Option<&ConfigType> {
        self.config_id2type.get(&id)
    }

    /// Return the default strategy of the configuration component with the given id.
    /// This is going to be `Some(..)` for configuration components,
    /// and `None` for all other components.
    pub fn default_strategy(&self, id: UserComponentId) -> Option<DefaultStrategy> {
        self.config_id2default_strategy.get(&id).copied()
    }

    /// Returns whether a configuration type should be included in the generated code
    /// even if it's unused.
    ///
    /// It's `None` for all other component kinds.
    pub fn include_if_unused(&self, id: UserComponentId) -> Option<bool> {
        self.config_id2include_if_unused.get(&id).copied()
    }

    /// Return the scope tree that was built from the application blueprint.
    pub fn scope_graph(&self) -> &ScopeGraph {
        &self.scope_graph
    }

    /// Return the ids of the middlewares that wrap around the request handler with the given id.
    ///
    /// It panics if the component with the given id is not a request handler.
    pub fn middleware_ids(&self, id: UserComponentId) -> &[UserComponentId] {
        &self.handler_id2middleware_ids[&id]
    }

    /// Return the lint overrides for this component, if any.
    pub fn lints(&self, id: UserComponentId) -> Option<&BTreeMap<Lint, LintSetting>> {
        self.id2lints.get(&id)
    }

    /// Return the ids of the error observers that must be invoked when something goes wrong
    /// in the request processing pipeline for this handler.
    ///
    /// It panics if the component with the given id is not a request handler.
    pub fn error_observer_ids(&self, id: UserComponentId) -> &[UserComponentId] {
        &self.handler_id2error_observer_ids[&id]
    }
}

/// We try to batch together the computation of the JSON documentation for all the crates that,
/// based on the information we have so far, will be needed to generate the application code.
///
/// This is not *necessary*, but it can turn out to be a significant performance improvement
/// for projects that pull in a lot of dependencies in the signature of their components.
fn precompute_crate_docs<'a, I, J>(
    krate_collection: &CrateCollection,
    paths: I,
    imported_package_ids: J,
    diagnostics: &DiagnosticSink,
) where
    I: Iterator<Item = &'a FQPath>,
    J: Iterator<Item = &'a PackageId>,
{
    let mut package_ids = IndexSet::new();
    for path in paths {
        path.collect_package_ids(&mut package_ids);
    }
    package_ids.extend(imported_package_ids);
    let package_ids = package_ids.iter().map(|&p| p.to_owned());

    if let Err(e) = krate_collection.bootstrap_collection(package_ids.clone()) {
        let e = anyhow::anyhow!(e).context(
            "I failed to compute the JSON documentation for one or more crates in the workspace.",
        );
        diagnostics.push(e.into_miette());
    }
}

impl std::ops::Index<UserComponentId> for UserComponentDb {
    type Output = UserComponent;

    fn index(&self, index: UserComponentId) -> &Self::Output {
        &self.component_interner[index]
    }
}
