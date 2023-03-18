use std::borrow::Cow;
use std::collections::BTreeMap;

use ahash::{HashMap, HashMapExt};
use bimap::BiHashMap;
use guppy::graph::PackageGraph;
use indexmap::IndexSet;
use miette::NamedSource;
use rustdoc_types::ItemEnum;
use syn::spanned::Spanned;

use pavex_builder::Lifecycle;

use crate::compiler::analyses::computations::{ComputationDb, ComputationId};
use crate::compiler::analyses::raw_user_components::{
    RawUserComponent, RawUserComponentDb, RawUserComponentId, RouterKey,
};
use crate::compiler::computation::{BorrowSharedReference, Computation, MatchResult};
use crate::compiler::constructors::{Constructor, ConstructorValidationError};
use crate::compiler::error_handlers::{ErrorHandler, ErrorHandlerValidationError};
use crate::compiler::interner::Interner;
use crate::compiler::request_handlers::{RequestHandler, RequestHandlerValidationError};
use crate::compiler::resolvers::CallableResolutionError;
use crate::compiler::traits::{assert_trait_is_implemented, MissingTraitImplementationError};
use crate::compiler::utils::{get_err_variant, get_ok_variant, is_result, process_framework_path};
use crate::diagnostic;
use crate::diagnostic::{
    convert_proc_macro_span, convert_rustdoc_span, AnnotatedSnippet, CallableType,
    CompilerDiagnostic, LocationExt, SourceSpanExt,
};
use crate::language::{
    Callable, ResolvedPath, ResolvedPathQualifiedSelf, ResolvedPathSegment, ResolvedPathType,
    ResolvedType, TypeReference,
};
use crate::rustdoc::CrateCollection;
use crate::utils::comma_separated_list;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Component {
    RequestHandler {
        raw_user_component_id: RawUserComponentId,
    },
    ErrorHandler {
        source_id: SourceId,
    },
    Constructor {
        source_id: SourceId,
    },
    Transformer {
        computation_id: ComputationId,
        transformed_component_id: ComponentId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub(crate) enum SourceId {
    ComputationId(ComputationId),
    UserComponentId(RawUserComponentId),
}

impl From<ComputationId> for SourceId {
    fn from(value: ComputationId) -> Self {
        Self::ComputationId(value)
    }
}

impl From<RawUserComponentId> for SourceId {
    fn from(value: RawUserComponentId) -> Self {
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

    pub(crate) fn into_owned(self) -> HydratedComponent<'static> {
        match self {
            HydratedComponent::Constructor(c) => HydratedComponent::Constructor(c.into_owned()),
            HydratedComponent::RequestHandler(r) => {
                HydratedComponent::RequestHandler(r.into_owned())
            }
            HydratedComponent::ErrorHandler(e) => {
                HydratedComponent::ErrorHandler(Cow::Owned(e.into_owned()))
            }
            HydratedComponent::Transformer(t) => HydratedComponent::Transformer(t.into_owned()),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ComponentDb {
    interner: Interner<Component>,
    err_ref_id2error_handler_id: HashMap<ComponentId, ComponentId>,
    fallible_id2match_ids: HashMap<ComponentId, (ComponentId, ComponentId)>,
    match_id2fallible_id: HashMap<ComponentId, ComponentId>,
    borrow_id2owned_id: BiHashMap<ComponentId, ComponentId>,
    id2transformer_ids: HashMap<ComponentId, IndexSet<ComponentId>>,
    id2lifecycle: HashMap<ComponentId, Lifecycle>,
    error_handler_id2error_handler: HashMap<ComponentId, ErrorHandler>,
    router: BTreeMap<RouterKey, ComponentId>,
    into_response: ResolvedPathType,
}

impl ComponentDb {
    pub fn build(
        raw_user_component_db: &RawUserComponentDb,
        computation_db: &mut ComputationDb,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Self {
        enum ErrorHandlerId {
            Id(ComponentId),
            // Used when the error handler failed to pass its own validation.
            // It allows us to keep track of the fact that an error *was* registered for a fallible
            // constructor/request handler, preventing us from reporting (incorrectly) that an error
            // handler was missing.
            UserId(RawUserComponentId),
        }
        let mut fallible_component_id2error_handler_id =
            HashMap::<RawUserComponentId, Option<ErrorHandlerId>>::new();
        let mut raw_user_component_id2component_id = HashMap::new();
        let into_response = {
            let into_response = process_framework_path(
                "pavex_runtime::response::IntoResponse",
                package_graph,
                krate_collection,
            );
            let ResolvedType::ResolvedPath(into_response) = into_response else { unreachable!() };
            into_response
        };

        let mut self_ = Self {
            interner: Interner::new(),
            err_ref_id2error_handler_id: Default::default(),
            fallible_id2match_ids: Default::default(),
            match_id2fallible_id: Default::default(),
            borrow_id2owned_id: Default::default(),
            id2transformer_ids: Default::default(),
            id2lifecycle: Default::default(),
            error_handler_id2error_handler: Default::default(),
            router: Default::default(),
            into_response,
        };

        for (raw_user_component_id, _) in raw_user_component_db.constructors() {
            let c: Computation = computation_db[raw_user_component_id].clone().into();
            match TryInto::<Constructor>::try_into(c) {
                Err(e) => {
                    Self::invalid_constructor(
                        e,
                        raw_user_component_id,
                        raw_user_component_db,
                        computation_db,
                        package_graph,
                        krate_collection,
                        diagnostics,
                    );
                }
                Ok(c) => {
                    let lifecycle = raw_user_component_db.get_lifecycle(raw_user_component_id);
                    let constructor_id = self_.interner.get_or_intern(Component::Constructor {
                        source_id: SourceId::UserComponentId(raw_user_component_id),
                    });
                    raw_user_component_id2component_id
                        .insert(raw_user_component_id, constructor_id);
                    self_
                        .id2lifecycle
                        .insert(constructor_id, lifecycle.to_owned());

                    self_.register_derived_constructors(constructor_id, computation_db);
                    if is_result(c.output_type()) && lifecycle != &Lifecycle::Singleton {
                        // We'll try to match all fallible constructors with an error handler later.
                        // We skip singletons since we don't "handle" errors when constructing them.
                        // They are just bubbled up to the caller by the function that builds
                        // the application state.
                        fallible_component_id2error_handler_id.insert(raw_user_component_id, None);
                    }
                }
            }
        }

        for (raw_user_component_id, raw_user_component) in raw_user_component_db.request_handlers()
        {
            let callable = &computation_db[raw_user_component_id];
            let RawUserComponent::RequestHandler { router_key, .. } = raw_user_component else {
                unreachable!()
            };
            match RequestHandler::new(Cow::Borrowed(callable)) {
                Err(e) => {
                    Self::invalid_request_handler(
                        e,
                        raw_user_component_id,
                        raw_user_component_db,
                        computation_db,
                        krate_collection,
                        package_graph,
                        diagnostics,
                    );
                }
                Ok(h) => {
                    let handler_id = self_.interner.get_or_intern(Component::RequestHandler {
                        raw_user_component_id,
                    });
                    raw_user_component_id2component_id.insert(raw_user_component_id, handler_id);
                    self_.router.insert(router_key.to_owned(), handler_id);
                    let lifecycle = Lifecycle::RequestScoped;
                    self_.id2lifecycle.insert(handler_id, lifecycle.clone());

                    if is_result(h.output_type()) {
                        // We'll try to match it with an error handler later.
                        fallible_component_id2error_handler_id.insert(raw_user_component_id, None);

                        // For each Result type, register two match transformers that de-structure
                        // `Result<T,E>` into `T` or `E`.
                        let m = MatchResult::match_result(h.output_type());
                        let (ok, err) = (m.ok, m.err);

                        let ok_id =
                            self_.add_synthetic_transformer(ok.into(), handler_id, computation_db);

                        // For each Result type register a match transformer that
                        // transforms `Result<T,E>` into `E`.
                        let err_id =
                            self_.add_synthetic_transformer(err.into(), handler_id, computation_db);
                        self_
                            .fallible_id2match_ids
                            .insert(handler_id, (ok_id, err_id));
                        self_.match_id2fallible_id.insert(ok_id, handler_id);
                        self_.match_id2fallible_id.insert(err_id, handler_id);
                    }
                }
            }
        }

        for (error_handler_raw_user_component_id, fallible_raw_user_component_id) in
            raw_user_component_db.iter().filter_map(|(id, c)| match c {
                RawUserComponent::ErrorHandler {
                    fallible_callable_identifiers_id,
                    ..
                } => Some((id, *fallible_callable_identifiers_id)),
                RawUserComponent::RequestHandler { .. } | RawUserComponent::Constructor { .. } => {
                    None
                }
            })
        {
            let lifecycle = raw_user_component_db.get_lifecycle(fallible_raw_user_component_id);
            if lifecycle == &Lifecycle::Singleton {
                Self::error_handler_for_a_singleton(
                    error_handler_raw_user_component_id,
                    fallible_raw_user_component_id,
                    raw_user_component_db,
                    package_graph,
                    diagnostics,
                );
                continue;
            }
            let fallible_callable = &computation_db[fallible_raw_user_component_id];
            if is_result(fallible_callable.output.as_ref().unwrap()) {
                let error_handler_callable = &computation_db[error_handler_raw_user_component_id];
                // Capture immediately that an error handler was registered for this fallible component.
                // We will later overwrite the associated id if it passes validation.
                fallible_component_id2error_handler_id.insert(
                    fallible_raw_user_component_id,
                    Some(ErrorHandlerId::UserId(error_handler_raw_user_component_id)),
                );
                match ErrorHandler::new(error_handler_callable.to_owned(), fallible_callable) {
                    Ok(e) => {
                        // This may be `None` if the fallible component failed to pass its own
                        // validation - e.g. the constructor callable was not deemed to be a valid
                        // constructor.
                        if let Some(fallible_component_id) =
                            raw_user_component_id2component_id.get(&fallible_raw_user_component_id)
                        {
                            let error_handler_id = self_.add_error_handler(
                                e,
                                *fallible_component_id,
                                lifecycle.to_owned(),
                                error_handler_raw_user_component_id.into(),
                                computation_db,
                            );
                            raw_user_component_id2component_id
                                .insert(error_handler_raw_user_component_id, error_handler_id);
                            fallible_component_id2error_handler_id.insert(
                                fallible_raw_user_component_id,
                                Some(ErrorHandlerId::Id(error_handler_id)),
                            );
                        }
                    }
                    Err(e) => {
                        Self::invalid_error_handler(
                            e,
                            error_handler_raw_user_component_id,
                            raw_user_component_db,
                            computation_db,
                            krate_collection,
                            package_graph,
                            diagnostics,
                        );
                    }
                };
            } else {
                Self::error_handler_for_infallible_component(
                    error_handler_raw_user_component_id,
                    fallible_raw_user_component_id,
                    raw_user_component_db,
                    package_graph,
                    diagnostics,
                );
            }
        }

        for (fallible_raw_user_component_id, error_handler_id) in
            fallible_component_id2error_handler_id
        {
            if error_handler_id.is_none() {
                Self::missing_error_handler(
                    fallible_raw_user_component_id,
                    raw_user_component_db,
                    package_graph,
                    diagnostics,
                );
            }
        }

        // We need to make sure that all output nodes return the same output type.
        // We do this by adding a "response transformer" node that converts the output type to a
        // common type - `pavex_runtime::response::Response`.
        let into_response_path = self_.into_response.resolved_path();
        let iter: Vec<_> = self_
            .interner
            .iter()
            .filter_map(|(id, c)| match c {
                Component::RequestHandler {
                    raw_user_component_id,
                }
                | Component::ErrorHandler {
                    // There are no error handlers with a `ComputationId` source at this stage.
                    source_id: SourceId::UserComponentId(raw_user_component_id),
                } => Some((id, *raw_user_component_id)),
                _ => None,
            })
            .collect();
        for (component_id, raw_user_component_id) in iter.into_iter() {
            // If the component is fallible, we want to attach the transformer to its Ok matcher.
            let component_id =
                if let Some((ok_id, _)) = self_.fallible_id2match_ids.get(&component_id) {
                    *ok_id
                } else {
                    component_id
                };
            let callable = &computation_db[raw_user_component_id];
            let output = callable.output.as_ref().unwrap();
            let output = if is_result(output) {
                get_ok_variant(output)
            } else {
                output
            }
            .to_owned();
            if let Err(e) =
                assert_trait_is_implemented(krate_collection, &output, &self_.into_response)
            {
                Self::invalid_response_type(
                    e,
                    &output,
                    raw_user_component_id,
                    raw_user_component_db,
                    package_graph,
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
            match computation_db.resolve_and_intern(krate_collection, &transformer_path, None) {
                Ok(callable_id) => {
                    self_.get_or_intern_transformer(callable_id, component_id, computation_db);
                }
                Err(e) => {
                    Self::cannot_handle_into_response_implementation(
                        e,
                        &output,
                        raw_user_component_id,
                        raw_user_component_db,
                        package_graph,
                        diagnostics,
                    );
                }
            }
        }

        self_
    }

    fn add_error_handler(
        &mut self,
        e: ErrorHandler,
        fallible_component_id: ComponentId,
        lifecycle: Lifecycle,
        source_id: SourceId,
        computation_db: &mut ComputationDb,
    ) -> ComponentId {
        let error_handler_id = self
            .interner
            .get_or_intern(Component::ErrorHandler { source_id });
        self.error_handler_id2error_handler
            .insert(error_handler_id, e);
        self.id2lifecycle.insert(error_handler_id, lifecycle);

        // Add an `E -> &E` transformer, otherwise we'll a missing link between the fallible
        // component and the error handler, since the latter takes a _reference_ to the error as
        // input parameter.
        let error_match_id = self.fallible_id2match_ids[&fallible_component_id].1;
        let error_type = self
            .hydrated_component(error_match_id, computation_db)
            .output_type()
            .to_owned();
        let error_ref_id = self.add_synthetic_transformer(
            BorrowSharedReference::new(error_type).into(),
            error_match_id,
            computation_db,
        );
        self.err_ref_id2error_handler_id
            .insert(error_ref_id, error_handler_id);

        error_handler_id
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
            let c: Computation<'_> = BorrowSharedReference::new(output_type).into();
            let borrow_id = self.add_synthetic_constructor(
                // It's fine to unwrap, since constructors are guaranteed to return a non-unit type.
                // Therefore we can be certain the borrowing that return type doesn't give a computation
                // that returns the unit type;
                c.try_into().unwrap(),
                lifecycle.to_owned(),
                computation_db,
            );
            self.borrow_id2owned_id.insert(borrow_id, constructor_id);
        }
        if let Ok(constructor) = constructor.as_fallible() {
            let m = constructor.matchers();
            let (ok, err) = (m.ok, m.err);

            // For each Result type, register a match constructor that transforms
            // `Result<T,E>` into `T`.
            let ok_id = self.add_synthetic_constructor(ok.into_owned(), lifecycle, computation_db);

            // For each Result type, register a match transformer that transforms `Result<T,E>` into `E`.
            let err_id = self.add_synthetic_transformer(err.into(), constructor_id, computation_db);
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
        let constructor_component = Component::Constructor {
            source_id: SourceId::ComputationId(callable_id),
        };
        let constructor_id = self.interner.get_or_intern(constructor_component);
        self.id2lifecycle.insert(constructor_id, lifecycle);
        self.register_derived_constructors(constructor_id, computation_db);
        Ok(constructor_id)
    }

    fn add_synthetic_transformer(
        &mut self,
        computation: Computation<'static>,
        transformed_id: ComponentId,
        computation_db: &mut ComputationDb,
    ) -> ComponentId {
        let computation_id = computation_db.get_or_intern(computation);
        self.get_or_intern_transformer(computation_id, transformed_id, computation_db)
    }

    pub fn get_or_intern_transformer(
        &mut self,
        callable_id: ComputationId,
        transformed_component_id: ComponentId,
        computation_db: &ComputationDb,
    ) -> ComponentId {
        let transformer = Component::Transformer {
            computation_id: callable_id,
            transformed_component_id,
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

        let is_borrow = matches!(
            &computation_db[callable_id],
            Computation::BorrowSharedReference(_)
        );
        if is_borrow {
            self.borrow_id2owned_id
                .insert(transformer_id, transformed_component_id);
        }
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
        if let Some(borrow_id) = self.borrow_id2owned_id.get_by_right(&component_id) {
            derived_ids.push(*borrow_id);
            derived_ids.extend(self.derived_component_ids(*borrow_id));
        }
        derived_ids
    }

    /// Given the id of a [`MatchResult`] component, return the id of the corresponding fallible
    /// component.
    #[track_caller]
    pub fn fallible_id(&self, match_component_id: ComponentId) -> ComponentId {
        self.match_id2fallible_id[&match_component_id]
    }

    /// Given the id of a [`BorrowSharedReference`] component, return the id of the corresponding
    /// component that returns the owned variant it borrows from.
    #[track_caller]
    pub fn owned_id(&self, borrow_component_id: ComponentId) -> ComponentId {
        self.borrow_id2owned_id
            .get_by_left(&borrow_component_id)
            .copied()
            .unwrap()
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
                    SourceId::ComputationId(id) => computation_db[*id].clone(),
                    SourceId::UserComponentId(id) => computation_db[*id].clone().into(),
                };
                Some((id, Constructor(computation)))
            }
        })
    }

    pub(crate) fn raw_user_component_id(&self, id: ComponentId) -> Option<RawUserComponentId> {
        match &self[id] {
            Component::Constructor {
                source_id: SourceId::UserComponentId(raw_user_component_id),
            }
            | Component::ErrorHandler {
                source_id: SourceId::UserComponentId(raw_user_component_id),
            }
            | Component::RequestHandler {
                raw_user_component_id,
            } => Some(*raw_user_component_id),
            Component::ErrorHandler {
                source_id: SourceId::ComputationId(_),
            }
            | Component::Constructor {
                source_id: SourceId::ComputationId(_),
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
            Component::RequestHandler {
                raw_user_component_id,
            } => {
                let callable = &computation_db[*raw_user_component_id];
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
                    SourceId::ComputationId(id) => computation_db[*id].clone(),
                    SourceId::UserComponentId(id) => computation_db[*id].clone().into(),
                };
                HydratedComponent::Constructor(Constructor(c))
            }
            Component::Transformer { computation_id, .. } => {
                let c = &computation_db[*computation_id];
                HydratedComponent::Transformer(c.clone())
            }
        }
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
    /// Replace all unassigned generic type parameters in the constructor component with id set to `id` with
    /// the concrete types specified in `bindings`.
    ///
    /// The newly "bound" component will be added to the component database and its id returned.
    ///
    /// The same process will be applied to all derived components (borrowed references,
    /// error handlers, etc.), recursively.
    ///
    /// # Panics
    ///
    /// Panics if the component with id `id` is not a constructor.
    pub fn bind_generic_type_parameters(
        &mut self,
        id: ComponentId,
        bindings: &HashMap<String, ResolvedType>,
        computation_db: &mut ComputationDb,
    ) -> ComponentId {
        fn _get_root_component_id(
            component_id: ComponentId,
            component_db: &ComponentDb,
            computation_db: &ComputationDb,
        ) -> ComponentId {
            let templated_component = component_db
                .hydrated_component(component_id, computation_db)
                .into_owned();
            let HydratedComponent::Constructor(constructor) = templated_component else { unreachable!() };
            match &constructor.0 {
                Computation::Callable(_) => component_id,
                Computation::MatchResult(_) => _get_root_component_id(
                    component_db.fallible_id(component_id),
                    component_db,
                    computation_db,
                ),
                Computation::BorrowSharedReference(_) => _get_root_component_id(
                    component_db.owned_id(component_id),
                    component_db,
                    computation_db,
                ),
            }
        }

        // We want to make sure we are binding the root component (i.e. a constructor registered
        // by the user), not a derived one. If not, we might have resolution issues when computing
        // the call graph for handlers where these derived components are used.
        let id = _get_root_component_id(id, self, computation_db);
        let HydratedComponent::Constructor(constructor) = self.hydrated_component(id, computation_db).into_owned() else { unreachable!() };
        let lifecycle = self.lifecycle(id).cloned().unwrap();
        let bound_computation = constructor
            .0
            .bind_generic_type_parameters(bindings)
            .into_owned();
        let bound_computation_id = computation_db.get_or_intern(bound_computation);
        let bound_component_id = self
            .get_or_intern_constructor(bound_computation_id, lifecycle.clone(), computation_db)
            .unwrap();
        // ^ This registers all "derived" constructors as well (borrowed references, matchers, etc.)
        // but it doesn't take care of the error handler, in case `id` pointed to a fallible constructor.
        // We need to do that manually.
        if let Some((_, err_match_id)) = self.fallible_id2match_ids.get(&id) {
            if let Some(err_ref_id) = self.borrow_id2owned_id.get_by_right(err_match_id) {
                let err_handler_id = self.err_ref_id2error_handler_id[err_ref_id];
                let HydratedComponent::ErrorHandler(error_handler) = self.hydrated_component(err_handler_id, computation_db) else { unreachable!() };

                // `bindings` contains the concrete types for all the unassigned generic
                // type parameters that appear in the signature of the constructor.
                // The error handler might itself have unassigned generic parameters that are
                // _equivalent_ to those in the constructor, but named differently.
                //
                // E.g.
                // - Constructor: `fn constructor<T>(x: u64) -> Result<T, Error<T>>`
                // - Error handler: `fn error_handler<S>(e: &Error<S>) -> Response`
                //
                // This little utility function "adapts" the bindings from the constructor to the
                // bindings required by the error handler.
                let error_handler_bindings = {
                    let ref_constructor_error_type = ResolvedType::Reference(TypeReference {
                        is_mutable: false,
                        is_static: false,
                        inner: Box::new(get_err_variant(constructor.output_type()).to_owned()),
                    });
                    let ref_error_handler_error_type = error_handler.error_type_ref();

                    let remapping = ref_constructor_error_type
                        .is_equivalent_to(ref_error_handler_error_type)
                        .unwrap();
                    let mut error_handler_bindings = HashMap::new();
                    for (generic, concrete) in bindings {
                        // `bindings` contains the concrete types for all the unassigned generic
                        // type parameters that appear in the signature of the constructor.
                        // It is not guaranteed that ALL those generic type parameters appear in the
                        // signature of the error handler, so we need to mindful here.
                        //
                        // E.g.
                        // - Constructor: `fn constructor<T>(x: u64) -> Result<T, Error>`
                        // - Error handler: `fn error_handler(e: &Error) -> Response`
                        if let Some(error_handler_generic) = remapping.get(generic.as_str()) {
                            error_handler_bindings
                                .insert((*error_handler_generic).to_owned(), concrete.clone());
                        }
                    }
                    error_handler_bindings
                };

                let bound_error_handler =
                    error_handler.bind_generic_type_parameters(&error_handler_bindings);
                let bound_error_component_id = self.add_error_handler(
                    bound_error_handler,
                    bound_component_id,
                    lifecycle,
                    bound_computation_id.into(),
                    computation_db,
                );

                // Finally, we need to bound the error handler's transformers.
                if let Some(transformer_ids) = self.transformer_ids(err_handler_id).cloned() {
                    for transformer_id in transformer_ids {
                        let HydratedComponent::Transformer(transformer) = self.hydrated_component(transformer_id, computation_db) else { unreachable!() };
                        let bound_transformer = transformer
                            .bind_generic_type_parameters(bindings)
                            .into_owned();
                        self.add_synthetic_transformer(
                            bound_transformer,
                            bound_error_component_id,
                            computation_db,
                        );
                    }
                }
            }
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

/// Utility functions to produce diagnostics.
impl ComponentDb {
    fn invalid_constructor(
        e: ConstructorValidationError,
        raw_user_component_id: RawUserComponentId,
        raw_user_component_db: &RawUserComponentDb,
        computation_db: &ComputationDb,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let location = raw_user_component_db.get_location(raw_user_component_id);
        let source = match location.source_file(package_graph) {
            Ok(s) => s,
            Err(e) => {
                diagnostics.push(e.into());
                return;
            }
        };
        let label = diagnostic::get_f_macro_invocation_span(&source, location)
            .map(|s| s.labeled("The constructor was registered here".into()));
        let diagnostic = match e {
            ConstructorValidationError::CannotFalliblyReturnTheUnitType
            | ConstructorValidationError::CannotReturnTheUnitType => {
                CompilerDiagnostic::builder(source, e)
                    .optional_label(label)
                    .build()
            }
            ConstructorValidationError::UnderconstrainedGenericParameters { ref parameters } => {
                fn get_definition_span(
                    callable: &Callable,
                    free_parameters: &IndexSet<String>,
                    krate_collection: &CrateCollection,
                    package_graph: &PackageGraph,
                ) -> Option<AnnotatedSnippet> {
                    let global_item_id = callable.source_coordinates.as_ref()?;
                    let item = krate_collection.get_type_by_global_type_id(global_item_id);
                    let definition_span = item.span.as_ref()?;
                    let source_contents = diagnostic::read_source_file(
                        &definition_span.filename,
                        &package_graph.workspace(),
                    )
                    .ok()?;
                    let span = convert_rustdoc_span(&source_contents, definition_span.to_owned());
                    let span_contents =
                        source_contents[span.offset()..(span.offset() + span.len())].to_string();
                    let (generic_params, output) = match &item.inner {
                        ItemEnum::Function(_) => {
                            if let Ok(item) = syn::parse_str::<syn::ItemFn>(&span_contents) {
                                (item.sig.generics.params, item.sig.output)
                            } else if let Ok(item) =
                                syn::parse_str::<syn::ImplItemMethod>(&span_contents)
                            {
                                (item.sig.generics.params, item.sig.output)
                            } else {
                                panic!("Could not parse as a function or method:\n{span_contents}")
                            }
                        }
                        _ => unreachable!(),
                    };

                    let mut labels = vec![];
                    let subject_verb = if generic_params.len() == 1 {
                        "it is"
                    } else {
                        "they are"
                    };
                    for param in generic_params {
                        if let syn::GenericParam::Type(ty) = param {
                            if free_parameters.contains(ty.ident.to_string().as_str()) {
                                labels.push(
                                    convert_proc_macro_span(&span_contents, ty.span())
                                        .labeled("I can't infer this..".into()),
                                );
                            }
                        }
                    }
                    let output_span = if let syn::ReturnType::Type(_, output_type) = &output {
                        output_type.span()
                    } else {
                        output.span()
                    };
                    labels.push(
                        convert_proc_macro_span(&span_contents, output_span)
                            .labeled(format!("..because {subject_verb} not used here")),
                    );
                    let source_path = definition_span.filename.to_str().unwrap();
                    Some(AnnotatedSnippet::new_with_labels(
                        NamedSource::new(source_path, span_contents),
                        labels,
                    ))
                }

                let callable = &computation_db[raw_user_component_id];
                let definition_snippet =
                    get_definition_span(callable, parameters, krate_collection, package_graph);
                let subject_verb = if parameters.len() == 1 {
                    "it is"
                } else {
                    "they are"
                };
                let free_parameters = if parameters.len() == 1 {
                    format!("`{}`", &parameters[0])
                } else {
                    let mut buffer = String::new();
                    comma_separated_list(&mut buffer, parameters.iter(), |p| format!("`{}`", p))
                        .unwrap();
                    buffer
                };
                let error = anyhow::anyhow!(e)
                    .context(
                        format!(
                            "I am not smart enough to figure out the concrete type for all the generic parameters in `{}`.\n\
                            I can only infer the type of an unassigned generic parameter if it appears in the output type returned by the constructor. This is \
                            not the case for {free_parameters}, since {subject_verb} only used by the input parameters.",
                            callable.path));
                CompilerDiagnostic::builder(source, error)
                    .optional_label(label)
                    .optional_additional_annotated_snippet(definition_snippet)
                    .help(
                        "Specify the concrete type(s) for the problematic \
                        generic parameter(s) when registering the constructor against the blueprint: \n\
                        |  bp.constructor(\n\
                        |    f!(my_crate::my_constructor::<ConcreteType>), \n\
                        |    ..\n\
                        |  )".into())
                    // ^ TODO: add a proper code snippet here, using the actual function that needs
                    //    to be amended instead of a made signature
                    .build()
            }
            ConstructorValidationError::NakedGenericOutputType {
                ref naked_parameter,
            } => {
                fn get_definition_span(
                    callable: &Callable,
                    krate_collection: &CrateCollection,
                    package_graph: &PackageGraph,
                ) -> Option<AnnotatedSnippet> {
                    let global_item_id = callable.source_coordinates.as_ref()?;
                    let item = krate_collection.get_type_by_global_type_id(global_item_id);
                    let definition_span = item.span.as_ref()?;
                    let source_contents = diagnostic::read_source_file(
                        &definition_span.filename,
                        &package_graph.workspace(),
                    )
                    .ok()?;
                    let span = convert_rustdoc_span(&source_contents, definition_span.to_owned());
                    let span_contents =
                        source_contents[span.offset()..(span.offset() + span.len())].to_string();
                    let output = match &item.inner {
                        ItemEnum::Function(_) => {
                            if let Ok(item) = syn::parse_str::<syn::ItemFn>(&span_contents) {
                                item.sig.output
                            } else if let Ok(item) =
                                syn::parse_str::<syn::ImplItemMethod>(&span_contents)
                            {
                                item.sig.output
                            } else {
                                panic!("Could not parse as a function or method:\n{span_contents}")
                            }
                        }
                        _ => unreachable!(),
                    };

                    let output_span = if let syn::ReturnType::Type(_, output_type) = &output {
                        output_type.span()
                    } else {
                        output.span()
                    };
                    let label = convert_proc_macro_span(&span_contents, output_span)
                        .labeled("The invalid output type".to_string());
                    let source_path = definition_span.filename.to_str().unwrap();
                    Some(AnnotatedSnippet::new(
                        NamedSource::new(source_path, span_contents),
                        label,
                    ))
                }

                let callable = &computation_db[raw_user_component_id];
                let definition_snippet =
                    get_definition_span(callable, krate_collection, package_graph);
                let msg = format!(
                    "You can't return a naked generic parameter from a constructor, like `{naked_parameter}` in `{}`.\n\
                    I don't take into account trait bounds when building your dependency graph. A constructor \
                    that returns a naked generic parameter is equivalent, in my eyes, to a constructor that can build \
                    **any** type, which is unlikely to be what you want!",
                    callable.path
                );
                let error = anyhow::anyhow!(e).context(msg);
                CompilerDiagnostic::builder(source, error)
                    .optional_label(label)
                    .optional_additional_annotated_snippet(definition_snippet)
                    .help(
                        "Can you return a concrete type as output? \n\
                        Or wrap the generic parameter in a non-generic container? \
                        For example, `T` in `Vec<T>` is not considered to be a naked parameter."
                            .into(),
                    )
                    .build()
            }
        };
        diagnostics.push(diagnostic.into());
    }

    fn invalid_request_handler(
        e: RequestHandlerValidationError,
        raw_user_component_id: RawUserComponentId,
        raw_user_component_db: &RawUserComponentDb,
        computation_db: &ComputationDb,
        krate_collection: &CrateCollection,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let location = raw_user_component_db.get_location(raw_user_component_id);
        let source = match location.source_file(package_graph) {
            Ok(s) => s,
            Err(e) => {
                diagnostics.push(e.into());
                return;
            }
        };
        let label = diagnostic::get_f_macro_invocation_span(&source, location)
            .map(|s| s.labeled("The request handler was registered here".into()));
        let diagnostic = match e {
            RequestHandlerValidationError::CannotReturnTheUnitType
            | RequestHandlerValidationError::CannotFalliblyReturnTheUnitType => {
                CompilerDiagnostic::builder(source, e)
                    .optional_label(label)
                    .build()
            }
            RequestHandlerValidationError::UnderconstrainedGenericParameters { ref parameters } => {
                fn get_definition_span(
                    callable: &Callable,
                    free_parameters: &IndexSet<String>,
                    krate_collection: &CrateCollection,
                    package_graph: &PackageGraph,
                ) -> Option<AnnotatedSnippet> {
                    let global_item_id = callable.source_coordinates.as_ref()?;
                    let item = krate_collection.get_type_by_global_type_id(global_item_id);
                    let definition_span = item.span.as_ref()?;
                    let source_contents = diagnostic::read_source_file(
                        &definition_span.filename,
                        &package_graph.workspace(),
                    )
                    .ok()?;
                    let span = convert_rustdoc_span(&source_contents, definition_span.to_owned());
                    let span_contents =
                        source_contents[span.offset()..(span.offset() + span.len())].to_string();
                    let generic_params = match &item.inner {
                        ItemEnum::Function(_) => {
                            if let Ok(item) = syn::parse_str::<syn::ItemFn>(&span_contents) {
                                item.sig.generics.params
                            } else if let Ok(item) =
                                syn::parse_str::<syn::ImplItemMethod>(&span_contents)
                            {
                                item.sig.generics.params
                            } else {
                                panic!("Could not parse as a function or method:\n{span_contents}")
                            }
                        }
                        _ => unreachable!(),
                    };

                    let mut labels = vec![];
                    for param in generic_params {
                        if let syn::GenericParam::Type(ty) = param {
                            if free_parameters.contains(ty.ident.to_string().as_str()) {
                                labels.push(
                                    convert_proc_macro_span(&span_contents, ty.span()).labeled(
                                        "The generic parameter without a concrete type".into(),
                                    ),
                                );
                            }
                        }
                    }
                    let source_path = definition_span.filename.to_str().unwrap();
                    Some(AnnotatedSnippet::new_with_labels(
                        NamedSource::new(source_path, span_contents),
                        labels,
                    ))
                }

                let callable = &computation_db[raw_user_component_id];
                let definition_snippet =
                    get_definition_span(callable, parameters, krate_collection, package_graph);
                let free_parameters = if parameters.len() == 1 {
                    format!("`{}`", &parameters[0])
                } else {
                    let mut buffer = String::new();
                    comma_separated_list(&mut buffer, parameters.iter(), |p| format!("`{}`", p))
                        .unwrap();
                    buffer
                };
                let verb = if parameters.len() == 1 { "does" } else { "do" };
                let plural = if parameters.len() == 1 { "" } else { "s" };
                let error = anyhow::anyhow!(e)
                    .context(
                        format!(
                            "I am not smart enough to figure out the concrete type for all the generic parameters in `{}`.\n\
                            There should no unassigned generic parameters in request handlers, but {free_parameters} {verb} \
                            not seem to have been assigned a concrete type.",
                            callable.path));
                CompilerDiagnostic::builder(source, error)
                    .optional_label(label)
                    .optional_additional_annotated_snippet(definition_snippet)
                    .help(
                        format!("Specify the concrete type{plural} for {free_parameters} when registering the request handler against the blueprint: \n\
                        |  bp.route(\n\
                        |    ..\n\
                        |    f!(my_crate::my_handler::<ConcreteType>), \n\
                        |  )"))
                    // ^ TODO: add a proper code snippet here, using the actual function that needs
                    //    to be amended instead of a made signature
                    .build()
            }
        };
        diagnostics.push(diagnostic.into());
    }

    fn invalid_response_type(
        e: MissingTraitImplementationError,
        output_type: &ResolvedType,
        raw_user_component_id: RawUserComponentId,
        raw_user_component_db: &RawUserComponentDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let location = raw_user_component_db.get_location(raw_user_component_id);
        let raw_user_component = &raw_user_component_db[raw_user_component_id];
        let callable_type = raw_user_component.callable_type();
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
            "I can't use the type returned by this {callable_type} to create an HTTP \
                response.\n\
                It doesn't implement `pavex_runtime::response::IntoResponse`."
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
        raw_user_component_id: RawUserComponentId,
        raw_user_component_db: &RawUserComponentDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let location = raw_user_component_db.get_location(raw_user_component_id);
        let raw_user_component = &raw_user_component_db[raw_user_component_id];
        let callable_type = raw_user_component.callable_type();
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
        raw_user_component_id: RawUserComponentId,
        raw_user_component_db: &RawUserComponentDb,
        computation_db: &ComputationDb,
        krate_collection: &CrateCollection,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let location = raw_user_component_db.get_location(raw_user_component_id);
        let source = match location.source_file(package_graph) {
            Ok(s) => s,
            Err(e) => {
                diagnostics.push(e.into());
                return;
            }
        };
        let label = diagnostic::get_f_macro_invocation_span(&source, location)
            .map(|s| s.labeled("The error handler was registered here".into()));
        let diagnostic = match &e {
            // TODO: Add a sub-diagnostic showing the error handler signature, highlighting with
            // a label the missing return type.
            ErrorHandlerValidationError::CannotReturnTheUnitType(_) |
            // TODO: Add a sub-diagnostic showing the error handler signature, highlighting with
            // a label the input types. Perhaps add a signature showing the signature of
            // the associate fallible handler, highlighting the output type.
            ErrorHandlerValidationError::DoesNotTakeErrorReferenceAsInput { .. } => {
                CompilerDiagnostic::builder(source, e)
                    .optional_label(label)
                    .build()
            }
            ErrorHandlerValidationError::UnderconstrainedGenericParameters { ref parameters, ref error_ref_input_index } => {
                fn get_definition_span(
                    callable: &Callable,
                    free_parameters: &IndexSet<String>,
                    error_ref_input_index: usize,
                    krate_collection: &CrateCollection,
                    package_graph: &PackageGraph,
                ) -> Option<AnnotatedSnippet> {
                    let global_item_id = callable.source_coordinates.as_ref()?;
                    let item = krate_collection.get_type_by_global_type_id(global_item_id);
                    let definition_span = item.span.as_ref()?;
                    let source_contents = diagnostic::read_source_file(
                        &definition_span.filename,
                        &package_graph.workspace(),
                    )
                        .ok()?;
                    let span = convert_rustdoc_span(&source_contents, definition_span.to_owned());
                    let span_contents =
                        source_contents[span.offset()..(span.offset() + span.len())].to_string();
                    let (generic_params, error_input) = match &item.inner {
                        ItemEnum::Function(_) => {
                            if let Ok(item) = syn::parse_str::<syn::ItemFn>(&span_contents) {
                                (item.sig.generics.params, item.sig.inputs[error_ref_input_index].clone())
                            } else if let Ok(item) =
                                syn::parse_str::<syn::ImplItemMethod>(&span_contents)
                            {
                                (item.sig.generics.params, item.sig.inputs[error_ref_input_index].clone())
                            } else {
                                panic!("Could not parse as a function or method:\n{span_contents}")
                            }
                        }
                        _ => unreachable!(),
                    };

                    let mut labels = vec![];
                    let subject_verb = if generic_params.len() == 1 {
                        "it is"
                    } else {
                        "they are"
                    };
                    for param in generic_params {
                        if let syn::GenericParam::Type(ty) = param {
                            if free_parameters.contains(ty.ident.to_string().as_str()) {
                                labels.push(
                                    convert_proc_macro_span(&span_contents, ty.span())
                                        .labeled("I can't infer this..".into()),
                                );
                            }
                        }
                    }
                    let error_input_span = error_input.span();
                    labels.push(
                        convert_proc_macro_span(&span_contents, error_input_span)
                            .labeled(format!("..because {subject_verb} not used here")),
                    );
                    let source_path = definition_span.filename.to_str().unwrap();
                    Some(AnnotatedSnippet::new_with_labels(
                        NamedSource::new(source_path, span_contents),
                        labels,
                    ))
                }

                let callable = &computation_db[raw_user_component_id];
                let definition_snippet =
                    get_definition_span(callable, parameters, *error_ref_input_index, krate_collection, package_graph);
                let subject_verb = if parameters.len() == 1 {
                    "it isn't"
                } else {
                    "they aren't"
                };
                let free_parameters = if parameters.len() == 1 {
                    format!("`{}`", &parameters[0])
                } else {
                    let mut buffer = String::new();
                    comma_separated_list(&mut buffer, parameters.iter(), |p| format!("`{}`", p)).unwrap();
                    buffer
                };
                let error = anyhow::anyhow!(e)
                    .context(
                        format!(
                            "I am not smart enough to figure out the concrete type for all the generic parameters in `{}`.\n\
                            I can only infer the type of an unassigned generic parameter if it appears in the error type processed by this error handler. This is \
                            not the case for {free_parameters}, since {subject_verb} used by the error type.",
                            callable.path));
                CompilerDiagnostic::builder(source, error)
                    .optional_label(label)
                    .optional_additional_annotated_snippet(definition_snippet)
                    .help(
                        "Specify the concrete type(s) for the problematic \
                        generic parameter(s) when registering the error handler against the blueprint: \n\
                        |  .error_handler(\n\
                        |    f!(my_crate::my_error_handler::<ConcreteType>)\n\
                        |  )".into())
                    // ^ TODO: add a proper code snippet here, using the actual function that needs
                    //    to be amended instead of a made signature
                    .build()
            }
        };
        diagnostics.push(diagnostic.into());
    }

    fn error_handler_for_infallible_component(
        error_handler_id: RawUserComponentId,
        fallible_id: RawUserComponentId,
        raw_user_component_db: &RawUserComponentDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let fallible_kind = raw_user_component_db[fallible_id].callable_type();
        let location = raw_user_component_db.get_location(error_handler_id);
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
            "You registered an error handler for a {} that doesn't return a `Result`.",
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
        error_handler_id: RawUserComponentId,
        fallible_id: RawUserComponentId,
        raw_user_component_db: &RawUserComponentDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        debug_assert_eq!(
            raw_user_component_db[fallible_id].callable_type(),
            CallableType::Constructor
        );
        let location = raw_user_component_db.get_location(error_handler_id);
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
            "You can't register an error handler for a singleton constructor. \n\
                If I fail to build a singleton, I bubble up the error - it doesn't get handled.",
        );
        let diagnostic = CompilerDiagnostic::builder(source, error)
            .optional_label(label)
            .help("Remove the error handler, it is not needed.".to_string())
            .build();
        diagnostics.push(diagnostic.into());
    }

    fn missing_error_handler(
        fallible_id: RawUserComponentId,
        raw_user_component_db: &RawUserComponentDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let fallible_kind = raw_user_component_db[fallible_id].callable_type();
        let location = raw_user_component_db.get_location(fallible_id);
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
