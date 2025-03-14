use crate::compiler::analyses::components::component::{Component, TransformerInfo};
use crate::compiler::analyses::components::{
    ConsumptionMode, HydratedComponent, InsertTransformer, SourceId,
    unregistered::UnregisteredComponent,
};
use crate::compiler::analyses::computations::{ComputationDb, ComputationId};
use crate::compiler::analyses::config_types::ConfigTypeDb;
use crate::compiler::analyses::framework_items::FrameworkItemDb;
use crate::compiler::analyses::into_error::register_error_new_transformer;
use crate::compiler::analyses::prebuilt_types::PrebuiltTypeDb;
use crate::compiler::analyses::user_components::{
    ScopeGraph, ScopeId, UserComponent, UserComponentDb, UserComponentId,
};
use crate::compiler::component::{
    Constructor, ConstructorValidationError, DefaultStrategy, ErrorHandler, ErrorObserver,
    PostProcessingMiddleware, PreProcessingMiddleware, RequestHandler, WrappingMiddleware,
};
use crate::compiler::computation::{Computation, MatchResult};
use crate::compiler::interner::Interner;
use crate::compiler::traits::assert_trait_is_implemented;
use crate::compiler::utils::{
    get_ok_variant, process_framework_callable_path, process_framework_path,
};
use crate::diagnostic::{OptionalLabeledSpanExt, OptionalSourceSpanExt, ParsedSourceFile};
use crate::language::{
    Callable, Lifetime, ResolvedPath, ResolvedPathQualifiedSelf, ResolvedPathSegment, ResolvedType,
    TypeReference,
};
use crate::rustdoc::CrateCollection;
use ahash::{HashMap, HashMapExt, HashSet};
use guppy::graph::PackageGraph;
use indexmap::IndexSet;
use pavex_bp_schema::{CloningStrategy, Lifecycle, Lint, LintSetting};
use pavex_cli_diagnostic::AnnotatedSource;
use std::borrow::Cow;
use std::collections::BTreeMap;

pub(crate) mod diagnostics;

pub(crate) type ComponentId = la_arena::Idx<Component>;

#[derive(Debug)]
pub(crate) struct ComponentDb {
    user_component_db: UserComponentDb,
    prebuilt_type_db: PrebuiltTypeDb,
    config_type_db: ConfigTypeDb,
    interner: Interner<Component>,
    match_err_id2error_handler_id: HashMap<ComponentId, ComponentId>,
    fallible_id2match_ids: HashMap<ComponentId, (ComponentId, ComponentId)>,
    match_id2fallible_id: HashMap<ComponentId, ComponentId>,
    id2transformer_ids: HashMap<ComponentId, IndexSet<ComponentId>>,
    id2lifecycle: HashMap<ComponentId, Lifecycle>,
    /// For each constructible component, determine if it can be cloned or not.
    ///
    /// Invariants: there is an entry for every constructor and prebuilt type.
    id2cloning_strategy: HashMap<ComponentId, CloningStrategy>,
    /// For each configuration type, determine if it should be defaulted or not.
    ///
    /// Invariants: there is an entry for every configuration type.
    config_id2default_strategy: HashMap<ComponentId, DefaultStrategy>,
    /// Associate each request handler with the ordered list of middlewares that wrap around it.
    ///
    /// Invariants: there is an entry for every single request handler.
    handler_id2middleware_ids: HashMap<ComponentId, Vec<ComponentId>>,
    /// Associate each request handler with the ordered list of error observer that
    /// must be invoked if something goes wrong.
    ///
    /// Invariants: there is an entry for every single request handler.
    handler_id2error_observer_ids: HashMap<ComponentId, Vec<ComponentId>>,
    /// Associate each transformer with additional metadata required to use it in call graphs
    /// and codegen.
    ///
    /// Invariants: there is an entry for every single transformer.
    transformer_id2info: HashMap<ComponentId, TransformerInfo>,
    /// Associate each error observer with the index of the error input in the list of its
    /// input parameters.
    ///
    /// Invariants: there is an entry for every single error observer.
    error_observer_id2error_input_index: HashMap<ComponentId, usize>,
    error_handler_id2error_handler: HashMap<ComponentId, ErrorHandler>,
    /// A mapping from the low-level [`UserComponentId`]s to the high-level [`ComponentId`]s.
    ///
    /// This is used to "lift" mappings that use [`UserComponentId`] into mappings that
    /// use [`ComponentId`]. In particular:
    ///
    /// - match error handlers with the respective fallible components after they have been
    ///   converted into components.
    /// - match request handlers with the sequence of middlewares that wrap around them.
    /// - convert the ids in the router.
    user_component_id2component_id: HashMap<UserComponentId, ComponentId>,
    /// For each scope, it stores the ordered list of error observers that should be
    /// invoked if a component fails in that scope.
    scope_ids_with_observers: Vec<ScopeId>,
    /// A switch to control if, when a fallible component is registered, [`ComponentDb`]
    /// should automatically register matcher components for its output type.
    autoregister_matchers: bool,
    /// The resolved type for `pavex::Error`.
    /// It's memoised here to avoid re-resolving it multiple times while analysing a single
    /// blueprint.
    pub(crate) pavex_error: ResolvedType,
    /// The resolved type for `pavex::Response`.
    /// It's memoised here to avoid re-resolving it multiple times while analysing a single
    /// blueprint.
    pub(crate) pavex_response: ResolvedType,
    /// The resolved type for `pavex::middleware::Processing`.
    /// It's memoised here to avoid re-resolving it multiple times while analysing a single
    /// blueprint.
    pub(crate) pavex_processing: ResolvedType,
    /// Users register constructors directly with a blueprint.
    /// From these user-provided constructors, we build **derived** constructors:
    /// - if a constructor is fallible,
    ///   we create a synthetic constructor to retrieve the Ok variant of its output type
    /// - if a constructor is generic, we create new synthetic constructors by binding its unassigned generic parameters
    ///   to concrete types
    ///
    /// This map holds an entry for each derived constructor.
    /// The value points to the original user-registered constructor it was derived from.
    ///
    /// This dependency relationship can be **indirect**—e.g. an Ok-matcher is derived from a fallible constructor
    /// which was in turn derived
    /// by binding the generic parameters of a user-registered constructor.
    /// The key for the Ok-matcher would point to the user-registered constructor in this scenario,
    /// not to the intermediate derived constructor.
    derived2user_registered: HashMap<ComponentId, ComponentId>,
    /// The id for all framework primitives—i.e. components coming from [`FrameworkItemDb`].
    framework_primitive_ids: HashSet<ComponentId>,
}

/// The `build` method and its auxiliary routines.
impl ComponentDb {
    #[tracing::instrument("Build component database", skip_all)]
    pub fn build(
        user_component_db: UserComponentDb,
        framework_item_db: &FrameworkItemDb,
        computation_db: &mut ComputationDb,
        prebuilt_type_db: PrebuiltTypeDb,
        config_type_db: ConfigTypeDb,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
    ) -> ComponentDb {
        // We only need to resolve these once.
        let pavex_error = process_framework_path("pavex::Error", krate_collection);
        let pavex_processing =
            process_framework_path("pavex::middleware::Processing", krate_collection);
        let pavex_response = process_framework_path("pavex::response::Response", krate_collection);
        let pavex_error_ref = {
            ResolvedType::Reference(TypeReference {
                lifetime: Lifetime::Elided,
                inner: Box::new(pavex_error.clone()),
                is_mutable: false,
            })
        };
        let pavex_noop_wrap_id = {
            let pavex_noop_wrap_callable = process_framework_callable_path(
                "pavex::middleware::wrap_noop",
                package_graph,
                krate_collection,
            );
            let pavex_noop_wrap_computation =
                Computation::Callable(Cow::Owned(pavex_noop_wrap_callable));
            computation_db.get_or_intern(pavex_noop_wrap_computation)
        };

        let mut self_ = Self {
            user_component_db,
            prebuilt_type_db,
            config_type_db,
            interner: Interner::new(),
            match_err_id2error_handler_id: Default::default(),
            fallible_id2match_ids: Default::default(),
            match_id2fallible_id: Default::default(),
            id2transformer_ids: Default::default(),
            id2lifecycle: Default::default(),
            id2cloning_strategy: Default::default(),
            config_id2default_strategy: Default::default(),
            handler_id2middleware_ids: Default::default(),
            handler_id2error_observer_ids: Default::default(),
            transformer_id2info: Default::default(),
            error_observer_id2error_input_index: Default::default(),
            error_handler_id2error_handler: Default::default(),
            user_component_id2component_id: Default::default(),
            scope_ids_with_observers: vec![],
            autoregister_matchers: false,
            pavex_error,
            pavex_response,
            pavex_processing,
            derived2user_registered: Default::default(),
            framework_primitive_ids: Default::default(),
        };

        {
            // Keep track of which components are fallible to emit a diagnostic
            // if they were not paired with an error handler.
            let mut needs_error_handler = IndexSet::new();

            self_.process_request_handlers(
                &mut needs_error_handler,
                computation_db,
                package_graph,
                krate_collection,
                diagnostics,
            );
            self_.process_wrapping_middlewares(
                &mut needs_error_handler,
                computation_db,
                package_graph,
                krate_collection,
                diagnostics,
            );
            self_.process_pre_processing_middlewares(
                &mut needs_error_handler,
                computation_db,
                package_graph,
                krate_collection,
                diagnostics,
            );
            self_.process_post_processing_middlewares(
                &mut needs_error_handler,
                computation_db,
                package_graph,
                krate_collection,
                diagnostics,
            );
            self_.compute_request2middleware_chain(pavex_noop_wrap_id, computation_db);

            // This **must** be invoked after request handlers and middlewares have been
            // processed, since it needs to determine which scopes have error observers
            // attached to them.
            self_.process_error_observers(
                &pavex_error_ref,
                computation_db,
                package_graph,
                krate_collection,
                diagnostics,
            );

            // We process the backlog of matchers that were not registered during the initial
            // registration phase for request handlers.
            self_.register_all_matchers(computation_db);
            // From this point onwards, all fallible components will automatically get matchers registered.
            // All error matchers will be automatically paired with a conversion into `pavex::error::Error` if needed,
            // based on the scope they belong to.
            self_.autoregister_matchers = true;

            self_.process_constructors(
                &mut needs_error_handler,
                computation_db,
                framework_item_db,
                package_graph,
                krate_collection,
                diagnostics,
            );

            self_.process_error_handlers(
                &mut needs_error_handler,
                computation_db,
                package_graph,
                krate_collection,
                diagnostics,
            );

            self_.process_prebuilt_types(computation_db);
            self_.process_config_types(computation_db);

            for fallible_id in needs_error_handler {
                Self::missing_error_handler(fallible_id, &self_.user_component_db, diagnostics);
            }
        }

        self_.add_into_response_transformers(computation_db, krate_collection, diagnostics);

        for (id, type_) in framework_item_db.iter() {
            let component_id = self_.get_or_intern(
                UnregisteredComponent::SyntheticConstructor {
                    computation_id: computation_db.get_or_intern(Constructor(
                        Computation::PrebuiltType(Cow::Owned(type_.clone())),
                    )),
                    scope_id: self_.scope_graph().root_scope_id(),
                    lifecycle: framework_item_db.lifecycle(id),
                    cloning_strategy: framework_item_db.cloning_strategy(id),
                    derived_from: None,
                },
                computation_db,
            );
            self_.framework_primitive_ids.insert(component_id);
        }

        // Add a synthetic constructor for the `pavex::middleware::Next` type.
        {
            let callable = process_framework_callable_path(
                "pavex::middleware::Next::new",
                package_graph,
                krate_collection,
            );
            let computation = Computation::Callable(Cow::Owned(callable));
            self_.get_or_intern(
                UnregisteredComponent::SyntheticConstructor {
                    computation_id: computation_db.get_or_intern(Constructor(computation)),
                    scope_id: self_.scope_graph().root_scope_id(),
                    lifecycle: Lifecycle::RequestScoped,
                    cloning_strategy: CloningStrategy::NeverClone,
                    derived_from: None,
                },
                computation_db,
            );
        }

        self_
    }

    /// Register error and ok matchers for all fallible components.
    fn register_all_matchers(&mut self, computation_db: &mut ComputationDb) {
        let ids: Vec<ComponentId> = self.interner.iter().map(|(id, _)| id).collect();
        for id in ids {
            self.register_matchers(id, computation_db);
        }
    }

    fn get_or_intern(
        &mut self,
        unregistered_component: UnregisteredComponent,
        computation_db: &mut ComputationDb,
    ) -> ComponentId {
        let id = self
            .interner
            .get_or_intern(unregistered_component.component());

        if let Some(user_component_id) = self.user_component_id(id) {
            self.user_component_id2component_id
                .insert(user_component_id, id);
        }

        self.id2lifecycle
            .insert(id, unregistered_component.lifecycle(self));

        {
            use crate::compiler::analyses::components::UnregisteredComponent::*;
            match unregistered_component {
                ErrorHandler {
                    error_matcher_id,
                    error_handler,
                    error_source_id,
                    ..
                } => {
                    self.match_err_id2error_handler_id
                        .insert(error_matcher_id, id);

                    // We have some error observers, so we may need to convert the concrete error type
                    // into pavex::Error.
                    let ResolvedType::Reference(error_ref) = error_handler.error_type_ref() else {
                        unreachable!()
                    };
                    if error_ref.inner.as_ref() != &self.pavex_error {
                        let scope_ids = self.scope_ids_with_observers.clone();
                        for scope_id in scope_ids {
                            register_error_new_transformer(
                                error_matcher_id,
                                self,
                                computation_db,
                                scope_id,
                            );
                        }
                    }

                    self.transformer_id2info.insert(
                        id,
                        TransformerInfo {
                            input_index: error_handler.error_input_index,
                            when_to_insert: InsertTransformer::Eagerly,
                            transformation_mode: ConsumptionMode::SharedBorrow,
                        },
                    );
                    self.id2transformer_ids
                        .entry(error_source_id)
                        .or_default()
                        .insert(id);

                    self.error_handler_id2error_handler
                        .insert(id, error_handler);
                }
                UserConstructor {
                    user_component_id, ..
                } => {
                    let cloning_strategy = self
                        .user_component_db
                        .get_cloning_strategy(user_component_id)
                        .unwrap();
                    self.id2cloning_strategy
                        .insert(id, cloning_strategy.to_owned());
                }
                SyntheticConstructor {
                    cloning_strategy,
                    derived_from,
                    ..
                } => {
                    self.id2cloning_strategy.insert(id, cloning_strategy);
                    if let Some(derived_from) = derived_from {
                        self.derived2user_registered.insert(
                            id,
                            self.derived2user_registered
                                .get(&derived_from)
                                .cloned()
                                .unwrap_or(derived_from),
                        );
                    }
                }
                Transformer {
                    when_to_insert,
                    transformation_mode,
                    transformed_component_id,
                    transformed_input_index,
                    ..
                } => {
                    self.transformer_id2info.insert(
                        id,
                        TransformerInfo {
                            input_index: transformed_input_index,
                            when_to_insert,
                            transformation_mode,
                        },
                    );
                    self.id2transformer_ids
                        .entry(transformed_component_id)
                        .or_default()
                        .insert(id);
                }
                ErrorObserver {
                    error_input_index, ..
                } => {
                    self.error_observer_id2error_input_index
                        .insert(id, error_input_index);
                }
                SyntheticWrappingMiddleware {
                    derived_from: Some(derived_from),
                    ..
                } => {
                    self.derived2user_registered.insert(
                        id,
                        self.derived2user_registered
                            .get(&derived_from)
                            .cloned()
                            .unwrap_or(derived_from),
                    );
                }
                UserPrebuiltType { user_component_id } => {
                    let user_component = &self.user_component_db[user_component_id];
                    let ty_ = &self.prebuilt_type_db[user_component_id];
                    let cloning_strategy = self
                        .user_component_db
                        .get_cloning_strategy(user_component_id)
                        .unwrap();
                    self.id2cloning_strategy
                        .insert(id, cloning_strategy.to_owned());
                    self.get_or_intern(
                        UnregisteredComponent::SyntheticConstructor {
                            computation_id: computation_db.get_or_intern(Constructor(
                                Computation::PrebuiltType(Cow::Owned(ty_.to_owned())),
                            )),
                            scope_id: user_component.scope_id(),
                            lifecycle: self.user_component_db.get_lifecycle(user_component_id),
                            cloning_strategy: cloning_strategy.to_owned(),
                            derived_from: Some(id),
                        },
                        computation_db,
                    );
                }
                UserConfigType { user_component_id } => {
                    let user_component = &self.user_component_db[user_component_id];
                    let config = &self.config_type_db[user_component_id];
                    let cloning_strategy = self
                        .user_component_db
                        .get_cloning_strategy(user_component_id)
                        .unwrap();
                    self.id2cloning_strategy.insert(id, *cloning_strategy);
                    let default_strategy = self
                        .user_component_db
                        .get_default_strategy(user_component_id)
                        .unwrap();
                    self.config_id2default_strategy
                        .insert(id, *default_strategy);
                    self.get_or_intern(
                        UnregisteredComponent::SyntheticConstructor {
                            computation_id: computation_db.get_or_intern(Constructor(
                                Computation::PrebuiltType(Cow::Owned(config.ty().to_owned())),
                            )),
                            scope_id: user_component.scope_id(),
                            lifecycle: self.user_component_db.get_lifecycle(user_component_id),
                            cloning_strategy: cloning_strategy.to_owned(),
                            derived_from: Some(id),
                        },
                        computation_db,
                    );
                }
                RequestHandler { .. }
                | SyntheticWrappingMiddleware { .. }
                | UserWrappingMiddleware { .. }
                | UserPostProcessingMiddleware { .. }
                | UserPreProcessingMiddleware { .. } => {}
            }
        }

        if self.autoregister_matchers {
            self.register_matchers(id, computation_db);
        }

        id
    }

    /// Register ok and err matchers for a component if it's fallible.
    fn register_matchers(&mut self, id: ComponentId, computation_db: &mut ComputationDb) {
        let component = self.hydrated_component(id, computation_db);
        let Some(output_type) = component.output_type() else {
            return;
        };
        if !output_type.is_result() || matches!(component, HydratedComponent::Transformer(..)) {
            return;
        }

        let m = MatchResult::match_result(output_type);
        let (ok, err) = (m.ok, m.err);

        // If the component is a constructor, the ok matcher is a constructor.
        // Otherwise it's a transformer.
        let ok_id = match self.hydrated_component(id, computation_db) {
            HydratedComponent::Constructor(_) => {
                let ok_computation_id = computation_db.get_or_intern(ok);

                self.get_or_intern(
                    UnregisteredComponent::SyntheticConstructor {
                        computation_id: ok_computation_id,
                        scope_id: self.scope_id(id),
                        lifecycle: self.lifecycle(id),
                        cloning_strategy: self.id2cloning_strategy[&id],
                        derived_from: Some(id),
                    },
                    computation_db,
                )
            }
            _ => self.add_synthetic_transformer(
                ok.into(),
                id,
                self.scope_id(id),
                InsertTransformer::Eagerly,
                ConsumptionMode::Move,
                0,
                computation_db,
            ),
        };

        let err_id = self.add_synthetic_transformer(
            err.into(),
            id,
            self.scope_id(id),
            InsertTransformer::Eagerly,
            ConsumptionMode::Move,
            0,
            computation_db,
        );

        self.fallible_id2match_ids.insert(id, (ok_id, err_id));
        self.match_id2fallible_id.insert(ok_id, id);
        self.match_id2fallible_id.insert(err_id, id);
    }

    /// Validate all user-registered constructors.
    /// We add their information to the relevant metadata stores.
    /// In particular, we keep track of their associated error handler, if one exists.
    fn process_constructors(
        &mut self,
        needs_error_handler: &mut IndexSet<UserComponentId>,
        computation_db: &mut ComputationDb,
        framework_item_db: &FrameworkItemDb,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
    ) {
        let constructor_ids = self
            .user_component_db
            .constructors()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        for user_component_id in constructor_ids {
            let c: Computation = computation_db[user_component_id].clone().into();
            match Constructor::new(
                c,
                &self.pavex_error,
                &self.pavex_response,
                framework_item_db,
            ) {
                Err(e) => {
                    Self::invalid_constructor(
                        e,
                        user_component_id,
                        &self.user_component_db,
                        computation_db,
                        package_graph,
                        krate_collection,
                        diagnostics,
                    );
                }
                Ok(c) => {
                    let constructor_id = self.get_or_intern(
                        UnregisteredComponent::UserConstructor { user_component_id },
                        computation_db,
                    );

                    if self.lifecycle(constructor_id) == Lifecycle::Singleton {
                        let output_type = c.output_type();
                        // We can't use references as singletons, unless they are `'static.`
                        if let ResolvedType::Reference(ref_type) = output_type {
                            if ref_type.lifetime != Lifetime::Static {
                                Self::non_static_reference_in_singleton(
                                    output_type,
                                    user_component_id,
                                    &self.user_component_db,
                                    diagnostics,
                                );
                            }
                        } else if output_type.has_implicit_lifetime_parameters()
                            || !output_type.named_lifetime_parameters().is_empty()
                        {
                            Self::non_static_lifetime_parameter_in_singleton(
                                output_type,
                                user_component_id,
                                &self.user_component_db,
                                diagnostics,
                            );
                        }
                    }

                    if c.is_fallible() && self.lifecycle(constructor_id) != Lifecycle::Singleton {
                        // We'll try to match all fallible constructors with an error handler later.
                        // We skip singletons since we don't "handle" errors when constructing them.
                        // They are just bubbled up to the caller by the function that builds
                        // the application state.
                        needs_error_handler.insert(user_component_id);
                    }
                }
            }
        }
    }

    /// Validate all user-registered prebuilt types.
    /// We add their information to the relevant metadata stores.
    fn process_prebuilt_types(&mut self, computation_db: &mut ComputationDb) {
        let ids = self
            .user_component_db
            .prebuilt_types()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        for user_component_id in ids {
            self.get_or_intern(
                UnregisteredComponent::UserPrebuiltType { user_component_id },
                computation_db,
            );
        }
    }

    /// Validate all user-registered config types.
    /// We add their information to the relevant metadata stores.
    fn process_config_types(&mut self, computation_db: &mut ComputationDb) {
        let ids = self
            .user_component_db
            .config_types()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        for user_component_id in ids {
            self.get_or_intern(
                UnregisteredComponent::UserConfigType { user_component_id },
                computation_db,
            );
        }
    }

    fn process_request_handlers(
        &mut self,
        needs_error_handler: &mut IndexSet<UserComponentId>,
        computation_db: &mut ComputationDb,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
    ) {
        let request_handler_ids = self
            .user_component_db
            .request_handlers()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        for user_component_id in request_handler_ids {
            let callable = &computation_db[user_component_id];
            match RequestHandler::new(Cow::Borrowed(callable)) {
                Err(e) => {
                    Self::invalid_request_handler(
                        e,
                        user_component_id,
                        &self.user_component_db,
                        computation_db,
                        krate_collection,
                        package_graph,
                        diagnostics,
                    );
                }
                Ok(_) => {
                    let id = self.get_or_intern(
                        UnregisteredComponent::RequestHandler { user_component_id },
                        computation_db,
                    );
                    if self.hydrated_component(id, computation_db).is_fallible() {
                        // We'll try to match it with an error handler later.
                        needs_error_handler.insert(user_component_id);
                    }
                }
            }
        }
    }

    fn process_wrapping_middlewares(
        &mut self,
        needs_error_handler: &mut IndexSet<UserComponentId>,
        computation_db: &mut ComputationDb,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
    ) {
        let wrapping_middleware_ids = self
            .user_component_db
            .wrapping_middlewares()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        for user_component_id in wrapping_middleware_ids {
            let callable = &computation_db[user_component_id];
            match WrappingMiddleware::new(Cow::Borrowed(callable)) {
                Err(e) => {
                    Self::invalid_wrapping_middleware(
                        e,
                        user_component_id,
                        &self.user_component_db,
                        computation_db,
                        krate_collection,
                        package_graph,
                        diagnostics,
                    );
                }
                Ok(_) => {
                    let id = self.get_or_intern(
                        UnregisteredComponent::UserWrappingMiddleware { user_component_id },
                        computation_db,
                    );
                    if self.hydrated_component(id, computation_db).is_fallible() {
                        // We'll try to match it with an error handler later.
                        needs_error_handler.insert(user_component_id);
                    }
                }
            }
        }
    }

    fn process_pre_processing_middlewares(
        &mut self,
        needs_error_handler: &mut IndexSet<UserComponentId>,
        computation_db: &mut ComputationDb,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
    ) {
        let middleware_ids = self
            .user_component_db
            .pre_processing_middlewares()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        for user_component_id in middleware_ids {
            let callable = &computation_db[user_component_id];
            match PreProcessingMiddleware::new(Cow::Borrowed(callable)) {
                Err(e) => {
                    Self::invalid_pre_processing_middleware(
                        e,
                        user_component_id,
                        &self.user_component_db,
                        computation_db,
                        krate_collection,
                        package_graph,
                        diagnostics,
                    );
                }
                Ok(_) => {
                    let id = self.get_or_intern(
                        UnregisteredComponent::UserPreProcessingMiddleware { user_component_id },
                        computation_db,
                    );
                    if self.hydrated_component(id, computation_db).is_fallible() {
                        // We'll try to match it with an error handler later.
                        needs_error_handler.insert(user_component_id);
                    }
                }
            }
        }
    }

    fn process_post_processing_middlewares(
        &mut self,
        needs_error_handler: &mut IndexSet<UserComponentId>,
        computation_db: &mut ComputationDb,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
    ) {
        let middleware_ids = self
            .user_component_db
            .post_processing_middlewares()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        for user_component_id in middleware_ids {
            let callable = &computation_db[user_component_id];
            match PostProcessingMiddleware::new(Cow::Borrowed(callable), &self.pavex_response) {
                Err(e) => {
                    Self::invalid_post_processing_middleware(
                        e,
                        user_component_id,
                        &self.user_component_db,
                        computation_db,
                        krate_collection,
                        package_graph,
                        diagnostics,
                    );
                }
                Ok(_) => {
                    let id = self.get_or_intern(
                        UnregisteredComponent::UserPostProcessingMiddleware { user_component_id },
                        computation_db,
                    );
                    if self.hydrated_component(id, computation_db).is_fallible() {
                        // We'll try to match it with an error handler later.
                        needs_error_handler.insert(user_component_id);
                    }
                }
            }
        }
    }

    fn process_error_observers(
        &mut self,
        pavex_error_ref: &ResolvedType,
        computation_db: &mut ComputationDb,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
    ) {
        let error_observer_ids = self
            .user_component_db
            .error_observers()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        for user_component_id in error_observer_ids {
            let user_component = &self.user_component_db[user_component_id];
            let callable = &computation_db[user_component_id];
            let UserComponent::ErrorObserver { .. } = user_component else {
                unreachable!()
            };
            match ErrorObserver::new(Cow::Borrowed(callable), pavex_error_ref) {
                Err(e) => {
                    Self::invalid_error_observer(
                        e,
                        user_component_id,
                        &self.user_component_db,
                        computation_db,
                        krate_collection,
                        package_graph,
                        diagnostics,
                    );
                }
                Ok(eo) => {
                    self.get_or_intern(
                        UnregisteredComponent::ErrorObserver {
                            user_component_id,
                            error_input_index: eo.error_input_index,
                        },
                        computation_db,
                    );
                }
            }
        }

        self.compute_request2error_observer_chain();

        let mut v = vec![];
        for component_id in self.request_handler_ids() {
            if self.handler_id2error_observer_ids[&component_id].is_empty() {
                continue;
            }
            v.push(self.scope_id(component_id));
            if let Some(middleware_ids) = self.handler_id2middleware_ids.get(&component_id) {
                for middleware_id in middleware_ids {
                    v.push(self.scope_id(*middleware_id));
                }
            }
        }
        self.scope_ids_with_observers = v;
    }

    fn process_error_handlers(
        &mut self,
        missing_error_handlers: &mut IndexSet<UserComponentId>,
        computation_db: &mut ComputationDb,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
    ) {
        let iter = self
            .user_component_db
            .iter()
            .filter_map(|(id, c)| {
                use crate::compiler::analyses::user_components::UserComponent::*;
                let ErrorHandler {
                    fallible_id: fallible_callable_identifiers_id,
                    ..
                } = c
                else {
                    return None;
                };
                Some((id, *fallible_callable_identifiers_id))
            })
            .collect::<Vec<_>>();
        for (error_handler_user_component_id, fallible_user_component_id) in iter {
            let lifecycle = self
                .user_component_db
                .get_lifecycle(fallible_user_component_id);
            if lifecycle == Lifecycle::Singleton {
                Self::error_handler_for_a_singleton(
                    error_handler_user_component_id,
                    fallible_user_component_id,
                    &self.user_component_db,
                    diagnostics,
                );
                continue;
            }
            let fallible_callable = &computation_db[fallible_user_component_id];
            if fallible_callable.is_fallible() {
                let error_handler_callable = &computation_db[error_handler_user_component_id];
                // Capture immediately that an error handler was registered for this fallible component.
                missing_error_handlers.shift_remove(&fallible_user_component_id);
                match ErrorHandler::new(
                    error_handler_callable.to_owned(),
                    fallible_callable,
                    &self.pavex_error,
                ) {
                    Ok(e) => {
                        // This may be `None` if the fallible component failed to pass its own
                        // validation—e.g. the constructor callable was not deemed to be a valid
                        // constructor.
                        if let Some(fallible_component_id) = self
                            .user_component_id2component_id
                            .get(&fallible_user_component_id)
                        {
                            let error_matcher_id = self
                                .fallible_id2match_ids
                                .get(fallible_component_id)
                                .unwrap()
                                .1;

                            let error_matcher =
                                self.hydrated_component(error_matcher_id, computation_db);
                            let fallible_error = error_matcher.output_type().unwrap();
                            let ResolvedType::Reference(error_ref) = e.error_type_ref() else {
                                unreachable!()
                            };

                            // The error handler doesn't use the concrete type
                            // returned by the fallible component,
                            // it targets Pavex's error type.
                            // We introduce a transformer to do the upcasting.
                            let error_source_id = if error_ref.inner.as_ref() == &self.pavex_error
                                && error_ref.inner.as_ref() != fallible_error
                            {
                                register_error_new_transformer(
                                    error_matcher_id,
                                    self,
                                    computation_db,
                                    self.scope_id(error_matcher_id),
                                )
                                .unwrap()
                            } else {
                                error_matcher_id
                            };

                            self.get_or_intern(
                                UnregisteredComponent::ErrorHandler {
                                    source_id: error_handler_user_component_id.into(),
                                    error_handler: e,
                                    error_matcher_id,
                                    error_source_id,
                                },
                                computation_db,
                            );
                        }
                    }
                    Err(e) => {
                        Self::invalid_error_handler(
                            e,
                            error_handler_user_component_id,
                            &self.user_component_db,
                            computation_db,
                            krate_collection,
                            package_graph,
                            diagnostics,
                        );
                    }
                };
            } else {
                Self::error_handler_for_infallible_component(
                    error_handler_user_component_id,
                    fallible_user_component_id,
                    &self.user_component_db,
                    diagnostics,
                );
            }
        }
    }

    /// Compute the middleware chain for each request handler that was successfully validated.
    /// The middleware chain only includes wrapping middlewares that were successfully validated.
    /// Invalid middlewares are ignored.
    fn compute_request2middleware_chain(
        &mut self,
        pavex_noop_wrap_id: ComputationId,
        computation_db: &mut ComputationDb,
    ) {
        let request_handler_ids = self
            .user_component_db
            .request_handlers()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        for request_handler_id in request_handler_ids {
            let Some(handler_component_id) = self
                .user_component_id2component_id
                .get(&request_handler_id)
                .cloned()
            else {
                continue;
            };
            // We add a synthetic no-op wrapping middleware to have a place where to
            // "attach" state that needs to be shared between the first non-trivial middleware
            // and its siblings (i.e. the pre- and post-processing middlewares around it).
            let noop_component_id = self.get_or_intern(
                UnregisteredComponent::SyntheticWrappingMiddleware {
                    computation_id: pavex_noop_wrap_id,
                    scope_id: self.scope_id(handler_component_id),
                    derived_from: None,
                },
                computation_db,
            );
            let mut middleware_chain = vec![noop_component_id];

            for middleware_id in self
                .user_component_db
                .get_middleware_ids(request_handler_id)
            {
                if let Some(middleware_component_id) =
                    self.user_component_id2component_id.get(middleware_id)
                {
                    middleware_chain.push(*middleware_component_id);
                }
            }

            self.handler_id2middleware_ids
                .insert(handler_component_id, middleware_chain);
        }
    }

    /// Compute the list of error observers for each request handler that was successfully validated.
    /// The list only includes error observers that were successfully validated.
    /// Invalid error observers are ignored.
    fn compute_request2error_observer_chain(&mut self) {
        for (request_handler_id, _) in self.user_component_db.request_handlers() {
            let Some(handler_component_id) =
                self.user_component_id2component_id.get(&request_handler_id)
            else {
                continue;
            };
            let mut chain = vec![];
            for id in self
                .user_component_db
                .get_error_observer_ids(request_handler_id)
            {
                if let Some(component_id) = self.user_component_id2component_id.get(id) {
                    chain.push(*component_id);
                }
            }
            self.handler_id2error_observer_ids
                .insert(*handler_component_id, chain);
        }
    }

    /// We need to make sure that all output nodes return the same output type.
    /// We do this by adding a "response transformer" node that converts the output type to a
    /// common type—`pavex::response::Response`.
    fn add_into_response_transformers(
        &mut self,
        computation_db: &mut ComputationDb,
        krate_collection: &CrateCollection,
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
    ) {
        let into_response = {
            let into_response =
                process_framework_path("pavex::response::IntoResponse", krate_collection);
            let ResolvedType::ResolvedPath(into_response) = into_response else {
                unreachable!()
            };
            into_response
        };
        let into_response_path = into_response.resolved_path();
        let iter: Vec<_> = self
            .interner
            .iter()
            .filter_map(|(id, c)| {
                use crate::compiler::analyses::components::component::Component::*;

                match c {
                    RequestHandler { .. }
                    | PostProcessingMiddleware { .. }
                    | WrappingMiddleware { .. } => Some((id, c.source_id())),
                    Transformer { source_id, .. } => {
                        if self.is_error_handler(id) {
                            Some((id, source_id.clone()))
                        } else {
                            None
                        }
                    }
                    PrebuiltType { .. }
                    | PreProcessingMiddleware { .. }
                    | ConfigType { .. }
                    | Constructor { .. }
                    | ErrorObserver { .. } => None,
                }
            })
            .collect();
        for (component_id, source_id) in iter.into_iter() {
            // If the component is fallible, we want to attach the transformer to its Ok matcher.
            let component_id =
                if let Some((ok_id, _)) = self.fallible_id2match_ids.get(&component_id) {
                    *ok_id
                } else {
                    component_id
                };
            let callable = match source_id {
                SourceId::ComputationId(computation_id, _) => {
                    let Computation::Callable(callable) = &computation_db[computation_id] else {
                        continue;
                    };
                    callable.clone()
                }
                SourceId::UserComponentId(user_component_id) => {
                    Cow::Borrowed(&computation_db[user_component_id])
                }
            };
            let output = callable.output.as_ref().unwrap();
            let output = if output.is_result() {
                get_ok_variant(output)
            } else {
                output
            }
            .to_owned();
            if let Err(e) = assert_trait_is_implemented(krate_collection, &output, &into_response) {
                if let SourceId::UserComponentId(user_component_id) = source_id {
                    Self::invalid_response_type(
                        e,
                        &output,
                        user_component_id,
                        &self.user_component_db,
                        diagnostics,
                    );
                }
                continue;
            }
            let mut transformer_segments = into_response_path.segments.clone();
            transformer_segments.push(ResolvedPathSegment {
                ident: "into_response".into(),
                generic_arguments: vec![],
            });
            let transformer_path = ResolvedPath {
                segments: transformer_segments,
                qualified_self: Some(ResolvedPathQualifiedSelf {
                    position: into_response_path.segments.len(),
                    type_: output.clone().into(),
                }),
                package_id: into_response_path.package_id.clone(),
            };
            match computation_db.resolve_and_intern(krate_collection, &transformer_path, None) {
                Ok(callable_id) => {
                    let transformer = UnregisteredComponent::Transformer {
                        computation_id: callable_id,
                        transformed_component_id: component_id,
                        transformation_mode: ConsumptionMode::Move,
                        transformed_input_index: 0,
                        scope_id: self.scope_id(component_id),
                        when_to_insert: InsertTransformer::Eagerly,
                    };
                    self.get_or_intern(transformer, computation_db);
                }
                Err(e) => {
                    if let SourceId::UserComponentId(user_component_id) = source_id {
                        Self::cannot_handle_into_response_implementation(
                            e,
                            &output,
                            user_component_id,
                            &self.user_component_db,
                            diagnostics,
                        );
                    }
                }
            }
        }
    }
}

impl ComponentDb {
    fn add_synthetic_transformer(
        &mut self,
        computation: Computation<'static>,
        transformed_id: ComponentId,
        scope_id: ScopeId,
        when_to_insert: InsertTransformer,
        consumption_mode: ConsumptionMode,
        transformed_input_index: usize,
        computation_db: &mut ComputationDb,
    ) -> ComponentId {
        let computation_id = computation_db.get_or_intern(computation);
        self.get_or_intern_transformer(
            computation_id,
            transformed_id,
            scope_id,
            when_to_insert,
            consumption_mode,
            transformed_input_index,
            computation_db,
        )
    }

    pub fn get_or_intern_constructor(
        &mut self,
        callable_id: ComputationId,
        lifecycle: Lifecycle,
        scope_id: ScopeId,
        cloning_strategy: CloningStrategy,
        computation_db: &mut ComputationDb,
        framework_item_db: &FrameworkItemDb,
        derived_from: Option<ComponentId>,
    ) -> Result<ComponentId, ConstructorValidationError> {
        let callable = computation_db[callable_id].to_owned();
        Constructor::new(
            callable,
            &self.pavex_error,
            &self.pavex_response,
            framework_item_db,
        )?;
        let constructor_component = UnregisteredComponent::SyntheticConstructor {
            lifecycle,
            computation_id: callable_id,
            scope_id,
            cloning_strategy,
            derived_from,
        };
        Ok(self.get_or_intern(constructor_component, computation_db))
    }

    /// Raw access, only used for synthetic constructors (in particular, `Next`'s state
    /// constructors).
    pub fn get_or_intern_constructor_without_validation(
        &mut self,
        callable_id: ComputationId,
        lifecycle: Lifecycle,
        scope_id: ScopeId,
        cloning_strategy: CloningStrategy,
        computation_db: &mut ComputationDb,
        derived_from: Option<ComponentId>,
    ) -> Result<ComponentId, ConstructorValidationError> {
        let constructor_component = UnregisteredComponent::SyntheticConstructor {
            lifecycle,
            computation_id: callable_id,
            scope_id,
            cloning_strategy,
            derived_from,
        };
        Ok(self.get_or_intern(constructor_component, computation_db))
    }

    pub fn get_or_intern_wrapping_middleware(
        &mut self,
        callable: Cow<'_, Callable>,
        scope_id: ScopeId,
        derived_from: ComponentId,
        computation_db: &mut ComputationDb,
    ) -> ComponentId {
        let computation = Computation::Callable(callable).into_owned();
        let computation_id = computation_db.get_or_intern(computation);
        let middleware_component = UnregisteredComponent::SyntheticWrappingMiddleware {
            computation_id,
            scope_id,
            derived_from: Some(derived_from),
        };

        self.get_or_intern(middleware_component, computation_db)
    }

    pub fn get_or_intern_transformer(
        &mut self,
        callable_id: ComputationId,
        transformed_component_id: ComponentId,
        scope_id: ScopeId,
        when_to_insert: InsertTransformer,
        consumption_mode: ConsumptionMode,
        transformed_input_index: usize,
        computation_db: &mut ComputationDb,
    ) -> ComponentId {
        let transformer = UnregisteredComponent::Transformer {
            computation_id: callable_id,
            transformed_component_id,
            transformation_mode: consumption_mode,
            scope_id,
            when_to_insert,
            transformed_input_index,
        };
        self.get_or_intern(transformer, computation_db)
    }

    /// Retrieve the lifecycle for a component.
    pub fn lifecycle(&self, id: ComponentId) -> Lifecycle {
        self.id2lifecycle[&id]
    }

    /// Retrieve the lint overrides for a component.
    pub fn lints(&self, id: ComponentId) -> Option<&BTreeMap<Lint, LintSetting>> {
        let user_component_id = self.user_component_id(id)?;
        self.user_component_db.get_lints(user_component_id)
    }

    /// The mapping from a low-level [`UserComponentId`] to its corresponding [`ComponentId`].
    pub fn user_component_id2component_id(&self) -> &HashMap<UserComponentId, ComponentId> {
        &self.user_component_id2component_id
    }

    /// Iterate over all the components in the database alongside their ids.
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (ComponentId, &Component)> + DoubleEndedIterator {
        self.interner.iter()
    }

    /// If the component is an error match node, return the id of the
    /// error handler designated to handle the error.
    /// Otherwise, return `None`.
    pub fn error_handler_id(&self, err_match_id: ComponentId) -> Option<&ComponentId> {
        self.match_err_id2error_handler_id.get(&err_match_id)
    }

    /// Returns `true` if the component is an error handler, `false` otherwise.
    pub fn is_error_handler(&self, id: ComponentId) -> bool {
        self.error_handler_id2error_handler.contains_key(&id)
    }

    /// Returns `true` if the component is a pre-processing middleware, `false` otherwise.
    pub fn is_pre_processing_middleware(&self, id: ComponentId) -> bool {
        matches!(self[id], Component::PreProcessingMiddleware { .. })
    }

    /// Returns `true` if the component is a wrapping middleware, `false` otherwise.
    pub fn is_wrapping_middleware(&self, id: ComponentId) -> bool {
        matches!(self[id], Component::WrappingMiddleware { .. })
    }

    /// Returns `true` if the component is a post-processing middleware, `false` otherwise.
    pub fn is_post_processing_middleware(&self, id: ComponentId) -> bool {
        matches!(self[id], Component::PostProcessingMiddleware { .. })
    }

    /// If the component is a request handler, return the ids of the middlewares that wrap around
    /// it.
    /// Otherwise, return `None`.
    pub fn middleware_chain(&self, handler_id: ComponentId) -> Option<&[ComponentId]> {
        self.handler_id2middleware_ids
            .get(&handler_id)
            .map(|v| &v[..])
    }

    /// If the component is a request handler, return the ids of the error observers that must be
    /// invoked when something goes wrong in the request processing pipeline.
    /// Otherwise, return `None`.
    pub fn error_observers(&self, handler_id: ComponentId) -> Option<&[ComponentId]> {
        self.handler_id2error_observer_ids
            .get(&handler_id)
            .map(|v| &v[..])
    }

    /// If transformations must be applied to the component, return their ids.
    /// Otherwise, return `None`.
    pub fn transformer_ids(&self, component_id: ComponentId) -> Option<&IndexSet<ComponentId>> {
        self.id2transformer_ids.get(&component_id)
    }

    /// If the component is fallible, return the id of the `MatchResult` components for the `Ok`
    /// and the `Err` variants.
    /// If the component is infallible, return `None`.
    pub fn match_ids(
        &self,
        fallible_component_id: ComponentId,
    ) -> Option<&(ComponentId, ComponentId)> {
        self.fallible_id2match_ids.get(&fallible_component_id)
    }

    /// Return the ids of the components that are derived from the given constructor.
    /// E.g. if the constructor is a fallible constructor, the derived components are the
    /// `MatchResult` components for the `Ok` and `Err` variants (and their respective
    /// derived components).
    /// If the constructor is a non-fallible constructor, the derived components are the
    /// `BorrowSharedReference` component.
    pub fn derived_component_ids(&self, component_id: ComponentId) -> Vec<ComponentId> {
        let mut derived_ids = Vec::new();
        if let Some(match_ids) = self.match_ids(component_id) {
            derived_ids.push(match_ids.0);
            derived_ids.push(match_ids.1);
            derived_ids.extend(self.derived_component_ids(match_ids.0));
            derived_ids.extend(self.derived_component_ids(match_ids.1));
        }
        derived_ids
    }

    /// Return the id of user-registered component that `component_id` was derived from
    /// (e.g. an Ok-matcher is derived from a fallible constructor or
    /// a bound constructor from a generic user-registered one).
    ///
    /// **It only works for constructors and middlewares**.
    pub fn derived_from(&self, component_id: &ComponentId) -> Option<ComponentId> {
        self.derived2user_registered.get(component_id).cloned()
    }

    /// Returns `true` if the component is a framework primitive (i.e. it comes from
    /// [`FrameworkItemDb`].
    /// `false` otherwise.
    pub fn is_framework_primitive(&self, component_id: &ComponentId) -> bool {
        self.framework_primitive_ids.contains(component_id)
    }

    /// Given the id of a [`MatchResult`] component, return the id of the corresponding fallible
    /// component.
    #[track_caller]
    pub fn fallible_id(&self, match_component_id: ComponentId) -> ComponentId {
        self.match_id2fallible_id[&match_component_id]
    }

    #[track_caller]
    /// Given the id of a component, return the corresponding [`CloningStrategy`].
    /// It panics if called for a non-constructor component.
    pub fn cloning_strategy(&self, component_id: ComponentId) -> CloningStrategy {
        self.id2cloning_strategy[&component_id]
    }

    #[track_caller]
    /// Given the id of a component, return the corresponding [`DefaultStrategy`].
    /// It panics if called for a component that's not a configuration type.
    pub fn default_strategy(&self, component_id: ComponentId) -> DefaultStrategy {
        self.config_id2default_strategy[&component_id]
    }

    /// Iterate over all constructors in the component database, either user-provided or synthetic.
    pub fn constructors<'a>(
        &'a self,
        computation_db: &'a ComputationDb,
    ) -> impl Iterator<Item = (ComponentId, Constructor<'a>)> {
        self.interner.iter().filter_map(|(id, c)| {
            let Component::Constructor { source_id } = c else {
                return None;
            };
            let computation = match source_id {
                SourceId::ComputationId(id, _) => computation_db[*id].clone(),
                SourceId::UserComponentId(id) => computation_db[*id].clone().into(),
            };
            Some((id, Constructor(computation)))
        })
    }

    /// Iterate over all the request handlers in the component database.
    pub fn request_handler_ids(&self) -> impl Iterator<Item = ComponentId> + '_ {
        self.interner.iter().filter_map(|(id, c)| {
            let Component::RequestHandler { .. } = c else {
                return None;
            };
            Some(id)
        })
    }

    pub(crate) fn user_component_id(&self, id: ComponentId) -> Option<UserComponentId> {
        match &self[id] {
            Component::Constructor {
                source_id: SourceId::UserComponentId(user_component_id),
            }
            | Component::WrappingMiddleware {
                source_id: SourceId::UserComponentId(user_component_id),
            }
            | Component::PostProcessingMiddleware {
                source_id: SourceId::UserComponentId(user_component_id),
            }
            | Component::PreProcessingMiddleware {
                source_id: SourceId::UserComponentId(user_component_id),
            }
            | Component::ErrorObserver { user_component_id }
            | Component::PrebuiltType { user_component_id }
            | Component::ConfigType { user_component_id }
            | Component::RequestHandler { user_component_id } => Some(*user_component_id),
            Component::Constructor {
                source_id: SourceId::ComputationId(..),
            }
            | Component::WrappingMiddleware {
                source_id: SourceId::ComputationId(..),
            }
            | Component::PostProcessingMiddleware {
                source_id: SourceId::ComputationId(..),
            }
            | Component::PreProcessingMiddleware {
                source_id: SourceId::ComputationId(..),
            }
            | Component::Transformer { .. } => None,
        }
    }

    pub(crate) fn hydrated_component<'a, 'b: 'a>(
        &'a self,
        id: ComponentId,
        computation_db: &'b ComputationDb,
    ) -> HydratedComponent<'a> {
        let component = &self[id];
        match component {
            Component::RequestHandler { user_component_id } => {
                let callable = &computation_db[*user_component_id];
                let request_handler = RequestHandler {
                    callable: Cow::Borrowed(callable),
                };
                HydratedComponent::RequestHandler(request_handler)
            }
            Component::PostProcessingMiddleware { source_id } => {
                let c = match source_id {
                    SourceId::ComputationId(id, _) => computation_db[*id].clone(),
                    SourceId::UserComponentId(id) => computation_db[*id].clone().into(),
                };
                let Computation::Callable(callable) = c else {
                    unreachable!()
                };
                let pp = PostProcessingMiddleware { callable };
                HydratedComponent::PostProcessingMiddleware(pp)
            }
            Component::PreProcessingMiddleware { source_id } => {
                let c = match source_id {
                    SourceId::ComputationId(id, _) => computation_db[*id].clone(),
                    SourceId::UserComponentId(id) => computation_db[*id].clone().into(),
                };
                let Computation::Callable(callable) = c else {
                    unreachable!()
                };
                let pp = PreProcessingMiddleware { callable };
                HydratedComponent::PreProcessingMiddleware(pp)
            }
            Component::WrappingMiddleware { source_id } => {
                let c = match source_id {
                    SourceId::ComputationId(id, _) => computation_db[*id].clone(),
                    SourceId::UserComponentId(id) => computation_db[*id].clone().into(),
                };
                let Computation::Callable(callable) = c else {
                    unreachable!()
                };
                let w = WrappingMiddleware { callable };
                HydratedComponent::WrappingMiddleware(w)
            }
            Component::Constructor { source_id } => {
                let c = match source_id {
                    SourceId::ComputationId(id, _) => computation_db[*id].clone(),
                    SourceId::UserComponentId(id) => computation_db[*id].clone().into(),
                };
                HydratedComponent::Constructor(Constructor(c))
            }
            Component::Transformer { source_id, .. } => {
                let c = match source_id {
                    SourceId::ComputationId(id, _) => computation_db[*id].clone(),
                    SourceId::UserComponentId(id) => computation_db[*id].clone().into(),
                };
                let info = &self.transformer_id2info[&id];
                HydratedComponent::Transformer(c.clone(), *info)
            }
            Component::ErrorObserver { user_component_id } => {
                let callable = &computation_db[*user_component_id];
                let error_observer = ErrorObserver {
                    callable: Cow::Borrowed(callable),
                    error_input_index: self.error_observer_id2error_input_index[&id],
                };
                HydratedComponent::ErrorObserver(error_observer)
            }
            Component::PrebuiltType { user_component_id } => {
                let ty = &self.prebuilt_type_db[*user_component_id];
                HydratedComponent::PrebuiltType(Cow::Borrowed(ty))
            }
            Component::ConfigType { user_component_id } => {
                let ty = &self.config_type_db[*user_component_id];
                HydratedComponent::ConfigType(ty.to_owned())
            }
        }
    }

    /// Return the [`UserComponentDb`] used as a seed for this component database.
    pub fn user_component_db(&self) -> &UserComponentDb {
        &self.user_component_db
    }

    /// Return the [`ScopeGraph`] that backs the [`ScopeId`]s for this component database.
    pub fn scope_graph(&self) -> &ScopeGraph {
        self.user_component_db.scope_graph()
    }

    /// Return the [`ScopeId`] of the given component.
    pub fn scope_id(&self, component_id: ComponentId) -> ScopeId {
        match &self[component_id] {
            Component::RequestHandler { user_component_id }
            | Component::PrebuiltType { user_component_id }
            | Component::ConfigType { user_component_id }
            | Component::ErrorObserver { user_component_id } => {
                self.user_component_db[*user_component_id].scope_id()
            }
            Component::Transformer { source_id, .. }
            | Component::WrappingMiddleware { source_id }
            | Component::PostProcessingMiddleware { source_id }
            | Component::PreProcessingMiddleware { source_id }
            | Component::Constructor { source_id } => match source_id {
                SourceId::ComputationId(_, scope_id) => *scope_id,
                SourceId::UserComponentId(id) => self.user_component_db[*id].scope_id(),
            },
        }
    }

    /// Return the source file where the component is defined, if possible.
    /// Its definition span within that file is annotated with the provided label.
    pub fn registration_span(
        &self,
        component_id: ComponentId,
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
        label: String,
    ) -> Option<AnnotatedSource<ParsedSourceFile>> {
        let user_id = self.user_component_id(component_id)?;
        let location = self.user_component_db().get_location(user_id);
        diagnostics.source(&location).map(|s| {
            crate::diagnostic::f_macro_span(s.source(), location)
                .labeled(label)
                .attach(s)
        })
    }
}

impl ComponentDb {
    /// Print to stdout a debug dump of the component database, primarily for debugging
    /// purposes.
    #[allow(unused)]
    pub(crate) fn debug_dump(&self, computation_db: &ComputationDb) {
        for (component_id, _) in self.iter() {
            println!(
                "Component id: {:?}\nHydrated component: {:?}\nLifecycle: {:?}",
                component_id,
                self.hydrated_component(component_id, computation_db),
                self.lifecycle(component_id)
            );

            println!("Matchers:");
            if let Some((ok_id, err_id)) = self.match_ids(component_id) {
                let matchers = format!(
                    "- Ok: {:?}\n- Err: {:?}",
                    self.hydrated_component(*ok_id, computation_db),
                    self.hydrated_component(*err_id, computation_db)
                );
                println!("{}", textwrap::indent(&matchers, "  "));
            }
            println!("Error handler:");
            if let Some(err_handler_id) = self.error_handler_id(component_id) {
                let error_handler = format!(
                    "{:?}",
                    self.hydrated_component(*err_handler_id, computation_db)
                );
                println!("{}", textwrap::indent(&error_handler, "  "));
            }
            println!("Transformers:");
            if let Some(transformer_ids) = self.transformer_ids(component_id) {
                let transformers = transformer_ids
                    .iter()
                    .map(|id| format!("- {:?}", self.hydrated_component(*id, computation_db)))
                    .collect::<Vec<_>>()
                    .join("\n");
                println!("{}", textwrap::indent(&transformers, "  "));
            }
            println!();
        }
    }
}

// All methods related to the logic for binding generic components.
impl ComponentDb {
    pub fn bind_generic_type_parameters(
        &mut self,
        id: ComponentId,
        bindings: &HashMap<String, ResolvedType>,
        computation_db: &mut ComputationDb,
        framework_item_db: &FrameworkItemDb,
    ) -> ComponentId {
        fn _get_root_component_id(
            component_id: ComponentId,
            component_db: &ComponentDb,
            computation_db: &ComputationDb,
        ) -> ComponentId {
            let templated_component = component_db
                .hydrated_component(component_id, computation_db)
                .into_owned();
            match templated_component {
                HydratedComponent::WrappingMiddleware(..) => component_id,
                // We want to make sure we are binding the root component (i.e. a constructor registered
                // by the user), not a derived one. If not, we might have resolution issues when computing
                // the call graph for handlers where these derived components are used.
                HydratedComponent::Constructor(constructor) => match &constructor.0 {
                    Computation::PrebuiltType(..) | Computation::Callable(..) => component_id,
                    Computation::MatchResult(..) => _get_root_component_id(
                        component_db.fallible_id(component_id),
                        component_db,
                        computation_db,
                    ),
                },
                HydratedComponent::RequestHandler(..)
                | HydratedComponent::PostProcessingMiddleware(..)
                | HydratedComponent::PreProcessingMiddleware(..)
                | HydratedComponent::ErrorObserver(..)
                | HydratedComponent::PrebuiltType(..)
                | HydratedComponent::ConfigType(..)
                | HydratedComponent::Transformer(..) => {
                    todo!()
                }
            }
        }

        let unbound_root_id = _get_root_component_id(id, self, computation_db);

        let bound_component_id = match self
            .hydrated_component(unbound_root_id, computation_db)
            .into_owned()
        {
            HydratedComponent::Constructor(constructor) => {
                let cloning_strategy = self.id2cloning_strategy[&unbound_root_id];
                let bound_computation = constructor
                    .0
                    .bind_generic_type_parameters(bindings)
                    .into_owned();
                let bound_computation_id = computation_db.get_or_intern(bound_computation);
                self.get_or_intern_constructor(
                    bound_computation_id,
                    self.lifecycle(unbound_root_id),
                    self.scope_id(unbound_root_id),
                    cloning_strategy,
                    computation_db,
                    framework_item_db,
                    Some(unbound_root_id),
                )
                .unwrap()
            }
            HydratedComponent::WrappingMiddleware(mw) => {
                let bound_callable = mw.callable.bind_generic_type_parameters(bindings);
                self.get_or_intern_wrapping_middleware(
                    Cow::Owned(bound_callable),
                    self.scope_id(unbound_root_id),
                    unbound_root_id,
                    computation_db,
                )
            }
            HydratedComponent::RequestHandler(_)
            | HydratedComponent::ErrorObserver(_)
            | HydratedComponent::PostProcessingMiddleware(_)
            | HydratedComponent::PreProcessingMiddleware(_)
            | HydratedComponent::PrebuiltType(_)
            | HydratedComponent::ConfigType(_)
            | HydratedComponent::Transformer(..) => {
                todo!()
            }
        };
        let bound_root_component_id = bound_component_id;

        let mut unbound_transformer_ids: Vec<_> = self
            .transformer_ids(unbound_root_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|id| (id, bound_component_id, bindings.to_owned()))
            .collect();
        while let Some((unbound_transformer_id, bound_transformed_id, bindings)) =
            unbound_transformer_ids.pop()
        {
            let Component::Transformer {
                transformed_component_id: unbound_transformed_id,
                ..
            } = self[unbound_transformer_id]
            else {
                unreachable!()
            };
            let HydratedComponent::Transformer(computation, info) = self
                .hydrated_component(unbound_transformer_id, computation_db)
                .into_owned()
            else {
                unreachable!()
            };

            // `bindings` contains the concrete types for all the unassigned generic
            // type parameters that appear in the signature of the transformed component.
            // The transformer might itself have unassigned generic parameters that are
            // _equivalent_ to those in the transformed component, but named differently.
            //
            // E.g.
            // - Constructor: `fn constructor<T>(x: u64) -> Result<T, Error<T>>`
            // - Error handler: `fn error_handler<S>(e: &Error<S>) -> Response`
            //
            // This little utility function "adapts" the bindings from the naming of the transformed
            // component to the ones required by the transformer.
            let transformer_bindings = {
                let unbound_transformed_output = self
                    .hydrated_component(unbound_transformed_id, computation_db)
                    .output_type()
                    .unwrap()
                    .to_owned();
                let transformer_input_type = &computation.input_types()[info.input_index];
                let unbound_transformed_output = match info.transformation_mode {
                    ConsumptionMode::Move => unbound_transformed_output,
                    ConsumptionMode::SharedBorrow => ResolvedType::Reference(TypeReference {
                        is_mutable: false,
                        lifetime: Lifetime::Elided,
                        inner: Box::new(unbound_transformed_output),
                    }),
                    ConsumptionMode::ExclusiveBorrow => ResolvedType::Reference(TypeReference {
                        is_mutable: true,
                        lifetime: Lifetime::Elided,
                        inner: Box::new(unbound_transformed_output),
                    }),
                };
                let remapping = unbound_transformed_output
                    .is_equivalent_to(transformer_input_type)
                    .unwrap_or_else(||
                        panic!(
                            "The transformed component's output type is not equivalent to the \
                            transformer's input type.\nTransformed component: {:?}\nTransformer: {:?}",
                            unbound_transformed_output, transformer_input_type
                        )
                    );
                let mut transformer_bindings = HashMap::new();
                for (generic, concrete) in bindings {
                    // `bindings` contains the concrete types for all the unassigned generic
                    // type parameters that appear in the signature of the transformed component.
                    // It is not guaranteed that ALL those generic type parameters appear in the
                    // signature of the transformer, so we need to mindful here.
                    //
                    // E.g.
                    // - Constructor: `fn constructor<T>(x: u64) -> Result<T, Error>`
                    // - Error handler: `fn error_handler(e: &Error) -> Response`
                    if let Some(transformer_generic) = remapping.get(generic.as_str()) {
                        transformer_bindings
                            .insert((*transformer_generic).to_owned(), concrete.clone());
                    }
                }
                transformer_bindings
            };

            let bound_transformer_computation = computation
                .bind_generic_type_parameters(&transformer_bindings)
                .into_owned();
            let bound_transformer_computation_id =
                computation_db.get_or_intern(bound_transformer_computation.clone());
            let unregistered_bound_component = if self.is_error_handler(unbound_transformer_id) {
                let Computation::Callable(bound_callable) = bound_transformer_computation.clone()
                else {
                    unreachable!()
                };
                let bound_error_handler = ErrorHandler {
                    callable: bound_callable.into_owned(),
                    error_input_index: info.input_index,
                };
                UnregisteredComponent::ErrorHandler {
                    source_id: SourceId::ComputationId(
                        bound_transformer_computation_id,
                        self.scope_id(unbound_transformer_id),
                    ),
                    // TODO: this is a hack. There will never be more than one error
                    //  handler in the transformer tree for a constructor/request_handler/middleware
                    //  If this assumption stops holding, this will misbehave badly.
                    error_matcher_id: self.match_ids(bound_root_component_id).unwrap().1,
                    error_source_id: bound_transformed_id,
                    error_handler: bound_error_handler,
                }
            } else {
                UnregisteredComponent::Transformer {
                    computation_id: bound_transformer_computation_id,
                    transformed_component_id: bound_transformed_id,
                    transformation_mode: info.transformation_mode,
                    transformed_input_index: info.input_index,
                    scope_id: self.scope_id(unbound_transformer_id),
                    when_to_insert: info.when_to_insert,
                }
            };
            let bound_transformer_id =
                self.get_or_intern(unregistered_bound_component, computation_db);

            unbound_transformer_ids.extend(
                self.transformer_ids(unbound_transformer_id)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|id| (id, bound_transformer_id, transformer_bindings.clone())),
            );
        }

        bound_component_id
    }
}

impl std::ops::Index<ComponentId> for ComponentDb {
    type Output = Component;

    fn index(&self, index: ComponentId) -> &Self::Output {
        &self.interner[index]
    }
}
