use std::borrow::Cow;
use std::collections::BTreeMap;

use ahash::{HashMap, HashMapExt};
use guppy::graph::PackageGraph;
use indexmap::IndexSet;

use pavex_builder::Lifecycle;

use crate::diagnostic;
use crate::diagnostic::{CallableType, CompilerDiagnostic, LocationExt, SourceSpanExt};
use crate::language::{ResolvedPath, ResolvedPathQualifiedSelf, ResolvedPathSegment, ResolvedType};
use crate::rustdoc::CrateCollection;
use crate::web::analyses::computations::{ComputationDb, ComputationId};
use crate::web::analyses::raw_identifiers::RawCallableIdentifiersDb;
use crate::web::analyses::user_components::{
    RouterKey, UserComponent, UserComponentDb, UserComponentId,
};
use crate::web::computation::{BorrowSharedReference, Computation, MatchResult};
use crate::web::constructors::{Constructor, ConstructorValidationError};
use crate::web::error_handlers::{ErrorHandler, ErrorHandlerValidationError};
use crate::web::interner::Interner;
use crate::web::request_handlers::{RequestHandler, RequestHandlerValidationError};
use crate::web::resolvers::CallableResolutionError;
use crate::web::traits::{assert_trait_is_implemented, MissingTraitImplementationError};
use crate::web::utils::{get_ok_variant, is_result, process_framework_path};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Component {
    RequestHandler { user_component_id: UserComponentId },
    ErrorHandler { user_component_id: UserComponentId },
    Constructor { source_id: ConstructorSourceId },
    Transformer { computation_id: ComputationId },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub(crate) enum ConstructorSourceId {
    ComputationId(ComputationId),
    UserComponentId(UserComponentId),
}

impl From<ComputationId> for ConstructorSourceId {
    fn from(value: ComputationId) -> Self {
        Self::ComputationId(value)
    }
}

impl From<UserComponentId> for ConstructorSourceId {
    fn from(value: UserComponentId) -> Self {
        Self::UserComponentId(value)
    }
}

pub(crate) type ComponentId = la_arena::Idx<Component>;

/// A transformation that, given a set of inputs, **constructs** a new type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum HydratedComponent<'a> {
    Constructor(Constructor<'a>),
    RequestHandler(RequestHandler<'a>),
    ErrorHandler(Cow<'a, ErrorHandler>),
    Transformer(Computation<'a>),
}

impl<'a> HydratedComponent<'a> {
    pub(crate) fn input_types(&self) -> Cow<[ResolvedType]> {
        match self {
            HydratedComponent::Constructor(c) => c.input_types(),
            HydratedComponent::RequestHandler(r) => Cow::Borrowed(r.input_types()),
            HydratedComponent::ErrorHandler(e) => Cow::Borrowed(e.input_types()),
            HydratedComponent::Transformer(c) => c.input_types(),
        }
    }

    pub(crate) fn output_type(&self) -> &ResolvedType {
        match self {
            HydratedComponent::Constructor(c) => c.output_type(),
            HydratedComponent::RequestHandler(r) => r.output_type(),
            HydratedComponent::ErrorHandler(e) => e.output_type(),
            // TODO: we are not enforcing that the output type of a transformer is not
            //  the unit type. In particular, you can successfully register a `Result<T, ()>`
            //  type, which will result into a `MatchResult` with output `()` for the error.
            HydratedComponent::Transformer(c) => c.output_type().unwrap(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ComponentDb {
    interner: Interner<Component>,
    err_ref_id2error_handler_id: HashMap<ComponentId, ComponentId>,
    fallible_id2match_ids: HashMap<ComponentId, (ComponentId, ComponentId)>,
    match_id2fallible_id: HashMap<ComponentId, ComponentId>,
    id2transformer_ids: HashMap<ComponentId, IndexSet<ComponentId>>,
    id2lifecycle: HashMap<ComponentId, Lifecycle>,
    error_handler_id2error_handler: HashMap<ComponentId, ErrorHandler>,
    router: BTreeMap<RouterKey, ComponentId>,
}

impl ComponentDb {
    pub fn build(
        user_component_db: &UserComponentDb,
        computation_db: &mut ComputationDb,
        package_graph: &PackageGraph,
        raw_identifiers_db: &RawCallableIdentifiersDb,
        krate_collection: &CrateCollection,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Self {
        enum ErrorHandlerId {
            Id(ComponentId),
            // Used when the error handler failed to pass its own validation.
            // It allows us to keep track of the fact that an error *was* registered for a fallible
            // constructor/request handler, preventing us from reporting (incorrectly) that an error
            // handler was missing.
            UserId(UserComponentId),
        }
        let mut fallible_component_id2error_handler_id =
            HashMap::<UserComponentId, Option<ErrorHandlerId>>::new();
        let mut user_component_id2component_id = HashMap::new();

        let mut self_ = Self {
            interner: Interner::new(),
            err_ref_id2error_handler_id: Default::default(),
            fallible_id2match_ids: Default::default(),
            match_id2fallible_id: Default::default(),
            id2transformer_ids: Default::default(),
            id2lifecycle: Default::default(),
            error_handler_id2error_handler: Default::default(),
            router: Default::default(),
        };

        for (user_component_id, user_component) in user_component_db
            .iter()
            .filter(|(_, c)| c.callable_type() == CallableType::Constructor)
        {
            let c: Computation = computation_db[user_component_id].clone().into();
            match TryInto::<Constructor>::try_into(c) {
                Err(e) => {
                    Self::invalid_constructor(
                        e,
                        user_component_id,
                        user_component_db,
                        package_graph,
                        raw_identifiers_db,
                        diagnostics,
                    );
                }
                Ok(c) => {
                    let lifecycle = raw_identifiers_db
                        .get_lifecycle(user_component.raw_callable_identifiers_id())
                        .unwrap();
                    let constructor_id = self_.interner.get_or_intern(Component::Constructor {
                        source_id: ConstructorSourceId::UserComponentId(user_component_id),
                    });
                    user_component_id2component_id.insert(user_component_id, constructor_id);
                    self_
                        .id2lifecycle
                        .insert(constructor_id, lifecycle.to_owned());

                    self_.register_derived_constructors(constructor_id, computation_db);
                    if is_result(c.output_type()) && lifecycle != &Lifecycle::Singleton {
                        // We'll try to match all fallible constructors with an error handler later.
                        // We skip singletons since we do not "handle" errors when constructing them.
                        // They are just bubbled up to the caller by the function that builds
                        // the application state.
                        fallible_component_id2error_handler_id.insert(user_component_id, None);
                    }
                }
            }
        }

        for (user_component_id, user_component) in user_component_db
            .iter()
            .filter(|(_, c)| c.callable_type() == CallableType::RequestHandler)
        {
            let callable = &computation_db[user_component_id];
            let UserComponent::RequestHandler { router_key, .. } = user_component else {
                unreachable!()
            };
            match RequestHandler::new(Cow::Borrowed(callable)) {
                Err(e) => {
                    Self::invalid_request_handler(
                        e,
                        user_component_id,
                        user_component_db,
                        package_graph,
                        raw_identifiers_db,
                        diagnostics,
                    );
                }
                Ok(h) => {
                    let handler_id = self_
                        .interner
                        .get_or_intern(Component::RequestHandler { user_component_id });
                    user_component_id2component_id.insert(user_component_id, handler_id);
                    self_.router.insert(router_key.to_owned(), handler_id);
                    let lifecycle = Lifecycle::RequestScoped;
                    self_.id2lifecycle.insert(handler_id, lifecycle.clone());

                    if is_result(h.output_type()) {
                        // We'll try to match it with an error handler later.
                        fallible_component_id2error_handler_id.insert(user_component_id, None);

                        // For each Result type, register two match transformers that de-structure
                        // `Result<T,E>` into `T` or `E`.
                        let m = MatchResult::match_result(h.output_type());
                        let (ok, err) = (m.ok, m.err);

                        let ok_id = self_.add_synthetic_transformer(
                            ok.into(),
                            handler_id,
                            lifecycle.clone(),
                            computation_db,
                        );

                        // For each Result type, register a match transformer that transforms
                        // `Result<T,E>` into `E`.
                        let err_id = self_.add_synthetic_transformer(
                            err.into(),
                            handler_id,
                            lifecycle,
                            computation_db,
                        );

                        self_
                            .fallible_id2match_ids
                            .insert(handler_id, (ok_id, err_id));
                        self_.match_id2fallible_id.insert(ok_id, handler_id);
                        self_.match_id2fallible_id.insert(err_id, handler_id);
                    }
                }
            }
        }

        for (error_handler_user_component_id, fallible_user_component_id) in
            user_component_db.iter().filter_map(|(id, c)| match c {
                UserComponent::ErrorHandler {
                    fallible_callable_identifiers_id,
                    ..
                } => Some((id, *fallible_callable_identifiers_id)),
                UserComponent::RequestHandler { .. } | UserComponent::Constructor { .. } => None,
            })
        {
            let lifecycle = raw_identifiers_db
                .get_lifecycle(
                    user_component_db[fallible_user_component_id].raw_callable_identifiers_id(),
                )
                .unwrap();
            if lifecycle == &Lifecycle::Singleton {
                Self::error_handler_for_a_singleton(
                    error_handler_user_component_id,
                    fallible_user_component_id,
                    user_component_db,
                    package_graph,
                    raw_identifiers_db,
                    diagnostics,
                );
                continue;
            }
            let fallible_callable = &computation_db[fallible_user_component_id];
            if is_result(fallible_callable.output.as_ref().unwrap()) {
                let error_handler_callable = &computation_db[error_handler_user_component_id];
                // Capture immediately that an error handler was registered for this fallible component.
                // We will later overwrite the associated id if it passes validation.
                fallible_component_id2error_handler_id.insert(
                    fallible_user_component_id,
                    Some(ErrorHandlerId::UserId(error_handler_user_component_id)),
                );
                match ErrorHandler::new(error_handler_callable.to_owned(), fallible_callable) {
                    Ok(e) => {
                        let error_handler_id =
                            self_.interner.get_or_intern(Component::ErrorHandler {
                                user_component_id: error_handler_user_component_id,
                            });
                        self_
                            .error_handler_id2error_handler
                            .insert(error_handler_id, e);
                        self_
                            .id2lifecycle
                            .insert(error_handler_id, lifecycle.to_owned());
                        user_component_id2component_id
                            .insert(error_handler_user_component_id, error_handler_id);
                        fallible_component_id2error_handler_id.insert(
                            fallible_user_component_id,
                            Some(ErrorHandlerId::Id(error_handler_id)),
                        );
                    }
                    Err(e) => {
                        Self::invalid_error_handler(
                            e,
                            error_handler_user_component_id,
                            user_component_db,
                            package_graph,
                            raw_identifiers_db,
                            diagnostics,
                        );
                    }
                };
            } else {
                Self::error_handler_for_infallible_component(
                    error_handler_user_component_id,
                    fallible_user_component_id,
                    user_component_db,
                    package_graph,
                    raw_identifiers_db,
                    diagnostics,
                );
            }
        }

        for (fallible_user_component_id, error_handler_id) in fallible_component_id2error_handler_id
        {
            match error_handler_id {
                None => {
                    Self::missing_error_handler(
                        fallible_user_component_id,
                        user_component_db,
                        package_graph,
                        raw_identifiers_db,
                        diagnostics,
                    );
                }
                Some(ErrorHandlerId::Id(error_handler_id)) => {
                    let fallible_component_id =
                        user_component_id2component_id[&fallible_user_component_id];
                    let err_match_id = self_.fallible_id2match_ids[&fallible_component_id].1;
                    let err_type = self_
                        .hydrated_component(err_match_id, computation_db)
                        .output_type()
                        .to_owned();
                    let err_ref_id = self_.add_synthetic_transformer(
                        BorrowSharedReference::new(err_type).into(),
                        err_match_id,
                        self_.id2lifecycle[&err_match_id].clone(),
                        computation_db,
                    );
                    self_
                        .err_ref_id2error_handler_id
                        .insert(err_ref_id, error_handler_id);
                }
                Some(ErrorHandlerId::UserId(_)) => {}
            }
        }

        // We need to make sure that all output nodes return the same output type.
        // We do this by adding a "response transformer" node that converts the output type to a
        // common type - `pavex_runtime::response::Response`.
        let into_response = process_framework_path(
            "pavex_runtime::response::IntoResponse",
            package_graph,
            krate_collection,
        );
        let ResolvedType::ResolvedPath(into_response) = into_response else { unreachable!() };
        let into_response_path = into_response.resolved_path();
        let iter: Vec<_> = self_
            .interner
            .iter()
            .filter_map(|(id, c)| match c {
                Component::RequestHandler { user_component_id }
                | Component::ErrorHandler { user_component_id } => Some((id, *user_component_id)),
                _ => None,
            })
            .collect();
        for (component_id, user_component_id) in iter.into_iter() {
            // If the component is fallible, we want to attach the transformer to its Ok matcher.
            let component_id =
                if let Some((ok_id, _)) = self_.fallible_id2match_ids.get(&component_id) {
                    *ok_id
                } else {
                    component_id
                };
            let callable = &computation_db[user_component_id];
            let output = callable.output.as_ref().unwrap();
            let output = if is_result(output) {
                get_ok_variant(output)
            } else {
                output
            }
            .to_owned();
            if let Err(e) = assert_trait_is_implemented(krate_collection, &output, &into_response) {
                Self::invalid_response_type(
                    e,
                    &output,
                    user_component_id,
                    user_component_db,
                    package_graph,
                    raw_identifiers_db,
                    diagnostics,
                );
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
            match computation_db.resolve_callable(krate_collection, &transformer_path, None) {
                Ok(callable_id) => {
                    self_.get_or_intern_transformer(callable_id, component_id);
                }
                Err(e) => {
                    Self::cannot_handle_into_response_implementation(
                        e,
                        &output,
                        user_component_id,
                        user_component_db,
                        package_graph,
                        raw_identifiers_db,
                        diagnostics,
                    );
                }
            }
        }

        self_
    }

    /// Retrieve the lifecycle for a component.
    pub fn lifecycle(&self, id: ComponentId) -> Option<&Lifecycle> {
        self.id2lifecycle.get(&id)
    }

    /// The mapping from a route to its dedicated request handler.
    pub fn router(&self) -> &BTreeMap<RouterKey, ComponentId> {
        &self.router
    }

    /// Iterate over all the components in the database alongside their ids.
    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (ComponentId, &Component)> + ExactSizeIterator + DoubleEndedIterator
    {
        self.interner.iter()
    }

    fn add_synthetic_constructor(
        &mut self,
        c: Constructor<'static>,
        l: Lifecycle,
        computation_db: &mut ComputationDb,
    ) -> ComponentId {
        let computation_id = computation_db.get_or_intern(c);
        let id = self.interner.get_or_intern(Component::Constructor {
            source_id: computation_id.into(),
        });
        self.id2lifecycle.insert(id, l);
        self.register_derived_constructors(id, computation_db);
        id
    }

    fn add_synthetic_transformer(
        &mut self,
        computation: Computation<'static>,
        transformed_id: ComponentId,
        l: Lifecycle,
        computation_db: &mut ComputationDb,
    ) -> ComponentId {
        let computation_id = computation_db.get_or_intern(computation);
        let id = self
            .interner
            .get_or_intern(Component::Transformer { computation_id });
        self.id2lifecycle.insert(id, l);
        self.id2transformer_ids
            .entry(transformed_id)
            .or_default()
            .insert(id);
        id
    }

    fn register_derived_constructors(
        &mut self,
        constructor_id: ComponentId,
        computation_db: &mut ComputationDb,
    ) {
        let constructor = {
            let HydratedComponent::Constructor(constructor) = self.hydrated_component(constructor_id, computation_db)
                else { unreachable!() };
            constructor.into_owned()
        };
        let output_type = constructor.output_type().to_owned();
        let lifecycle = self.lifecycle(constructor_id).unwrap().to_owned();
        if !matches!(output_type, ResolvedType::Reference(_)) {
            // For each non-reference type, register an inlineable constructor that transforms
            // `T` in `&T`.
            let c: Computation<'_> = BorrowSharedReference::new(output_type.to_owned()).into();
            self.add_synthetic_constructor(
                // It's fine to unwrap, since constructors are guaranteed to return a non-unit type.
                // Therefore we can be certain the borrowing that return type doesn't give a computation
                // that returns the unit type;
                c.try_into().unwrap(),
                lifecycle.to_owned(),
                computation_db,
            );
        }
        if let Ok(constructor) = constructor.as_fallible() {
            let m = constructor.matchers();
            let (ok, err) = (m.ok, m.err);

            // For each Result type, register a match constructor that transforms
            // `Result<T,E>` into `T`.
            let ok_id = self.add_synthetic_constructor(
                ok.into_owned(),
                lifecycle.to_owned(),
                computation_db,
            );

            // For each Result type, register a match transformer that transforms
            // `Result<T,E>` into `E`.
            let err: Computation = err.into();
            let err_id = self.add_synthetic_transformer(
                err.into_owned(),
                constructor_id,
                lifecycle,
                computation_db,
            );

            self.fallible_id2match_ids
                .insert(constructor_id, (ok_id, err_id));
            self.match_id2fallible_id.insert(ok_id, constructor_id);
            self.match_id2fallible_id.insert(err_id, constructor_id);
        }
    }

    pub fn get_or_intern_constructor(
        &mut self,
        callable_id: ComputationId,
        lifecycle: Lifecycle,
        computation_db: &mut ComputationDb,
    ) -> Result<ComponentId, ConstructorValidationError> {
        let callable = computation_db[callable_id].to_owned();
        TryInto::<Constructor>::try_into(callable)?;
        let constructor_id = self.interner.get_or_intern(Component::Constructor {
            source_id: ConstructorSourceId::ComputationId(callable_id),
        });
        self.id2lifecycle.insert(constructor_id, lifecycle);
        self.register_derived_constructors(constructor_id, computation_db);
        Ok(constructor_id)
    }

    pub fn get_or_intern_transformer(
        &mut self,
        callable_id: ComputationId,
        transformed_component_id: ComponentId,
    ) -> ComponentId {
        let transformer = Component::Transformer {
            computation_id: callable_id,
        };
        let transformer_id = self.interner.get_or_intern(transformer);
        self.id2transformer_ids
            .entry(transformed_component_id)
            .or_default()
            .insert(transformer_id);
        self.id2lifecycle.insert(
            transformer_id,
            self.lifecycle(transformed_component_id).unwrap().to_owned(),
        );
        transformer_id
    }

    /// If the component is an error reference, return the id of the
    /// error handler designated to handle the error.
    /// Otherwise, return `None`.
    pub fn error_handler_id(&self, err_ref_id: ComponentId) -> Option<&ComponentId> {
        self.err_ref_id2error_handler_id.get(&err_ref_id)
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

    /// Given the id of a `MatchResult` component, return the id of the corresponding fallible
    /// component.
    pub fn fallible_id(&self, match_component_id: ComponentId) -> ComponentId {
        self.match_id2fallible_id[&match_component_id]
    }

    /// Iterate over all constructors in the component database, either user-provided or synthetic.
    pub fn constructors<'a>(
        &'a self,
        computation_db: &'a ComputationDb,
    ) -> impl Iterator<Item = (ComponentId, Constructor<'a>)> {
        self.interner.iter().filter_map(|(id, c)| match c {
            Component::RequestHandler { .. }
            | Component::ErrorHandler { .. }
            | Component::Transformer { .. } => None,
            Component::Constructor { source_id } => {
                let computation = match source_id {
                    ConstructorSourceId::ComputationId(id) => computation_db[*id].clone(),
                    ConstructorSourceId::UserComponentId(id) => computation_db[*id].clone().into(),
                };
                Some((id, Constructor(computation)))
            }
        })
    }

    pub(crate) fn user_component_id(&self, id: ComponentId) -> Option<UserComponentId> {
        match &self[id] {
            Component::Constructor { source_id } => match source_id {
                ConstructorSourceId::ComputationId(_id) => None,
                ConstructorSourceId::UserComponentId(id) => Some(*id),
            },
            Component::ErrorHandler { user_component_id }
            | Component::RequestHandler { user_component_id } => Some(*user_component_id),
            Component::Transformer { .. } => None,
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
            Component::ErrorHandler { .. } => {
                let error_handler = &self.error_handler_id2error_handler[&id];
                HydratedComponent::ErrorHandler(Cow::Borrowed(error_handler))
            }
            Component::Constructor { source_id } => {
                let c = match source_id {
                    ConstructorSourceId::ComputationId(id) => computation_db[*id].clone(),
                    ConstructorSourceId::UserComponentId(id) => computation_db[*id].clone().into(),
                };
                HydratedComponent::Constructor(Constructor(c))
            }
            Component::Transformer { computation_id } => {
                let c = &computation_db[*computation_id];
                HydratedComponent::Transformer(c.clone())
            }
        }
    }
}

impl std::ops::Index<ComponentId> for ComponentDb {
    type Output = Component;

    fn index(&self, index: ComponentId) -> &Self::Output {
        &self.interner[index]
    }
}

/// Utility functions to produce diagnostics.
impl ComponentDb {
    fn invalid_constructor(
        e: ConstructorValidationError,
        user_component_id: UserComponentId,
        user_component_db: &UserComponentDb,
        package_graph: &PackageGraph,
        raw_identifiers_db: &RawCallableIdentifiersDb,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        match e {
            ConstructorValidationError::CannotFalliblyReturnTheUnitType
            | ConstructorValidationError::CannotReturnTheUnitType => {
                let user_component = &user_component_db[user_component_id];
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
                    .map(|s| s.labeled("The constructor was registered here".into()));
                let diagnostic = CompilerDiagnostic::builder(source, e)
                    .optional_label(label)
                    .build();
                diagnostics.push(diagnostic.into());
            }
        }
    }

    fn invalid_request_handler(
        e: RequestHandlerValidationError,
        user_component_id: UserComponentId,
        user_component_db: &UserComponentDb,
        package_graph: &PackageGraph,
        raw_identifiers_db: &RawCallableIdentifiersDb,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        match e {
            RequestHandlerValidationError::CannotReturnTheUnitType => {
                let user_component = &user_component_db[user_component_id];
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
                    .map(|s| s.labeled("The request handler was registered here".into()));
                let diagnostic = CompilerDiagnostic::builder(source, e)
                    .optional_label(label)
                    .build();
                diagnostics.push(diagnostic.into());
            }
        }
    }

    fn invalid_response_type(
        e: MissingTraitImplementationError,
        output_type: &ResolvedType,
        user_component_id: UserComponentId,
        user_component_db: &UserComponentDb,
        package_graph: &PackageGraph,
        raw_identifiers_db: &RawCallableIdentifiersDb,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let user_component = &user_component_db[user_component_id];
        let raw_identifier_id = user_component.raw_callable_identifiers_id();
        let callable_type = user_component.callable_type();
        let location = raw_identifiers_db.get_location(raw_identifier_id);
        let source = match location.source_file(package_graph) {
            Ok(s) => s,
            Err(e) => {
                diagnostics.push(e.into());
                return;
            }
        };
        let label = diagnostic::get_f_macro_invocation_span(&source, location)
            .map(|s| s.labeled(format!("The {callable_type} was registered here")));
        let error = anyhow::Error::from(e).context(format!(
            "I cannot use the type returned by this {callable_type} to create an HTTP \
                response.\n\
                It does not implement `pavex_runtime::response::IntoResponse`."
        ));
        let help =
            format!("Implement `pavex_runtime::response::IntoResponse` for `{output_type:?}`.");
        let diagnostic = CompilerDiagnostic::builder(source, error)
            .optional_label(label)
            .help(help)
            .build();
        diagnostics.push(diagnostic.into());
    }

    fn cannot_handle_into_response_implementation(
        e: CallableResolutionError,
        output_type: &ResolvedType,
        user_component_id: UserComponentId,
        user_component_db: &UserComponentDb,
        package_graph: &PackageGraph,
        raw_identifiers_db: &RawCallableIdentifiersDb,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let user_component = &user_component_db[user_component_id];
        let raw_identifier_id = user_component.raw_callable_identifiers_id();
        let callable_type = user_component.callable_type();
        let location = raw_identifiers_db.get_location(raw_identifier_id);
        let source = match location.source_file(package_graph) {
            Ok(s) => s,
            Err(e) => {
                diagnostics.push(e.into());
                return;
            }
        };
        let label = diagnostic::get_f_macro_invocation_span(&source, location)
            .map(|s| s.labeled(format!("The {callable_type} was registered here")));
        let error = anyhow::Error::from(e).context(format!(
            "Something went wrong when I tried to analyze the implementation of \
                `pavex_runtime::response::IntoResponse` for {output_type:?}, the type returned by 
                one of your {callable_type}s.\n\
                This is definitely a bug, I am sorry! Please file an issue on \
                https://github.com/LukeMathWalker/pavex"
        ));
        let diagnostic = CompilerDiagnostic::builder(source, error)
            .optional_label(label)
            .build();
        diagnostics.push(diagnostic.into());
    }

    fn invalid_error_handler(
        e: ErrorHandlerValidationError,
        user_component_id: UserComponentId,
        user_component_db: &UserComponentDb,
        package_graph: &PackageGraph,
        raw_identifiers_db: &RawCallableIdentifiersDb,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let user_component = &user_component_db[user_component_id];
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
            .map(|s| s.labeled("The error handler was registered here".into()));
        match &e {
            ErrorHandlerValidationError::CannotReturnTheUnitType(_) => {
                // TODO: Add a sub-diagnostic showing the error handler signature, highlighting with
                // a label the missing return type.
            }
            ErrorHandlerValidationError::DoesNotTakeErrorReferenceAsInput { .. } => {
                // TODO: Add a sub-diagnostic showing the error handler signature, highlighting with
                // a label the input types. Perhaps add a signature showing the signature of
                // the associate fallible handler, highlighting the output type.
            }
        }
        let diagnostic = CompilerDiagnostic::builder(source, e)
            .optional_label(label)
            .build();
        diagnostics.push(diagnostic.into());
    }

    fn error_handler_for_infallible_component(
        error_handler_id: UserComponentId,
        fallible_id: UserComponentId,
        user_component_db: &UserComponentDb,
        package_graph: &PackageGraph,
        raw_identifiers_db: &RawCallableIdentifiersDb,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let fallible_kind = user_component_db[fallible_id].callable_type();
        let user_component = &user_component_db[error_handler_id];
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
            .map(|s| s.labeled("The unnecessary error handler was registered here".into()));
        let error = anyhow::anyhow!(
            "You registered an error handler for a {} that does not return a `Result`.",
            fallible_kind
        );
        let diagnostic = CompilerDiagnostic::builder(source, error)
            .optional_label(label)
            .help(format!(
                "Remove the error handler, it is not needed. The {fallible_kind} is infallible!"
            ))
            .build();
        diagnostics.push(diagnostic.into());
    }

    fn error_handler_for_a_singleton(
        error_handler_id: UserComponentId,
        fallible_id: UserComponentId,
        user_component_db: &UserComponentDb,
        package_graph: &PackageGraph,
        raw_identifiers_db: &RawCallableIdentifiersDb,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let user_component = &user_component_db[error_handler_id];
        let raw_identifier_id = user_component.raw_callable_identifiers_id();
        debug_assert_eq!(
            user_component_db[fallible_id].callable_type(),
            CallableType::Constructor
        );
        let location = raw_identifiers_db.get_location(raw_identifier_id);
        let source = match location.source_file(package_graph) {
            Ok(s) => s,
            Err(e) => {
                diagnostics.push(e.into());
                return;
            }
        };
        let label = diagnostic::get_f_macro_invocation_span(&source, location)
            .map(|s| s.labeled("The unnecessary error handler was registered here".into()));
        let error = anyhow::anyhow!(
            "You cannot register an error handler for a singleton constructor. \n\
                If I fail to build a singleton, I bubble up the error - it does not get handled.",
        );
        let diagnostic = CompilerDiagnostic::builder(source, error)
            .optional_label(label)
            .help("Remove the error handler, it is not needed.".to_string())
            .build();
        diagnostics.push(diagnostic.into());
    }

    fn missing_error_handler(
        fallible_id: UserComponentId,
        user_component_db: &UserComponentDb,
        package_graph: &PackageGraph,
        raw_identifiers_db: &RawCallableIdentifiersDb,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let fallible_kind = user_component_db[fallible_id].callable_type();
        let user_component = &user_component_db[fallible_id];
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
            .map(|s| s.labeled(format!("The fallible {fallible_kind} was registered here")));
        let error = anyhow::anyhow!(
                "You registered a {fallible_kind} that returns a `Result`, but you did not register an \
                 error handler for it. \
                 If I don't have an error handler, I don't know what to do with the error when the \
                 {fallible_kind} fails!",
            );
        let diagnostic = CompilerDiagnostic::builder(source, error)
            .optional_label(label)
            .help("Add an error handler via `.error_handler`".to_string())
            .build();
        diagnostics.push(diagnostic.into());
    }
}
