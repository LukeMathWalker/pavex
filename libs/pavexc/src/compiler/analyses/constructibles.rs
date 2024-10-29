use std::collections::VecDeque;

use ahash::{HashMap, HashMapExt, HashSet};
use guppy::graph::PackageGraph;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use syn::spanned::Spanned;

use pavex_bp_schema::{CloningStrategy, Lifecycle};

use crate::compiler::analyses::components::{ComponentDb, ComponentId};
use crate::compiler::analyses::components::{ConsumptionMode, HydratedComponent};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::user_components::{
    ScopeGraph, ScopeId, UserComponentDb, UserComponentId,
};
use crate::compiler::computation::Computation;
use crate::diagnostic::{self, CallableDefinition, OptionalSourceSpanExt};
use crate::diagnostic::{AnnotatedSnippet, CompilerDiagnostic, HelpWithSnippet, SourceSpanExt};
use crate::language::{Callable, ResolvedType};
use crate::rustdoc::CrateCollection;
use crate::try_source;

use super::framework_items::FrameworkItemDb;

/// The set of types that can be injected into request handlers, error handlers and (other) constructors.
pub(crate) struct ConstructibleDb {
    scope_id2constructibles: IndexMap<ScopeId, ConstructiblesInScope>,
}

impl std::fmt::Debug for ConstructibleDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Available constructibles:\n")?;
        for (scope_id, constructibles) in &self.scope_id2constructibles {
            writeln!(
                f,
                "- {scope_id}:\n{}",
                // TODO: Use a PadAdapter down here to avoid allocating an intermediate string
                textwrap::indent(&format!("{:?}", constructibles), "    ")
            )?;
        }
        Ok(())
    }
}

impl ConstructibleDb {
    /// Compute the set of types that can be injected into request handlers, error handlers and
    /// (other) constructors.
    ///
    /// Emits diagnostics for any missing constructors.
    #[tracing::instrument("Build constructibles database", skip_all)]
    pub(crate) fn build(
        component_db: &mut ComponentDb,
        computation_db: &mut ComputationDb,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        framework_items_db: &FrameworkItemDb,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Self {
        let mut self_ = Self::_build(component_db, computation_db);
        self_.detect_missing_constructors(
            component_db,
            computation_db,
            package_graph,
            krate_collection,
            framework_items_db,
            diagnostics,
        );
        self_.verify_singleton_ambiguity(component_db, computation_db, package_graph, diagnostics);
        self_.verify_lifecycle_of_singleton_dependencies(
            component_db,
            computation_db,
            package_graph,
            diagnostics,
        );
        self_.error_observers_cannot_depend_on_fallible_components(
            component_db,
            computation_db,
            package_graph,
            diagnostics,
        );

        self_
    }

    fn _build(component_db: &ComponentDb, computation_db: &ComputationDb) -> Self {
        let mut self_ = Self {
            scope_id2constructibles: IndexMap::new(),
        };
        for (component_id, _) in component_db.constructors(computation_db) {
            self_.insert(component_id, component_db, computation_db);
        }
        self_
    }

    /// Check if any component is asking for a type as input parameter for which there is no
    /// constructor.
    fn detect_missing_constructors(
        &mut self,
        component_db: &mut ComponentDb,
        computation_db: &mut ComputationDb,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        framework_items_db: &FrameworkItemDb,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let mut component_ids = component_db.iter().map(|(id, _)| id).collect::<Vec<_>>();
        let mut n_component_ids = component_ids.len();
        // In the process of detecting missing constructors,
        // we might generate _new_ constructors by specializing generic constructors.
        // The newly generated constructors might themselves be missing constructors, so we
        // add them to set of components to check and iterate until we no longer have any new
        // components to check.
        loop {
            for component_id in component_ids {
                let scope_id = component_db.scope_id(component_id);
                let resolved_component =
                    component_db.hydrated_component(component_id, computation_db);
                let input_types = {
                    let mut input_types: Vec<Option<ResolvedType>> = resolved_component
                        .input_types()
                        .iter()
                        .map(|i| Some(i.to_owned()))
                        .collect();
                    match &resolved_component {
                        // `Next` is a special case: it's not a pre-determined type, but rather
                        // an ad-hoc type that is constructed by the framework at compile-time,
                        // specific to each request handling chain.
                        HydratedComponent::WrappingMiddleware(mw) => {
                            input_types[mw.next_input_index()] = None;
                        }
                        HydratedComponent::PostProcessingMiddleware(pp) => {
                            input_types[pp.response_input_index(&component_db.pavex_response)] =
                                None;
                        }
                        HydratedComponent::ErrorObserver(eo) => {
                            input_types[eo.error_input_index] = None;
                        }
                        HydratedComponent::Transformer(_, info) => {
                            // The transformer is only invoked when the transformed component is
                            // present in the graph, so we don't need to check for the transformed
                            // component's constructor.
                            input_types[info.input_index] = None;
                        }
                        HydratedComponent::Constructor(_)
                        | HydratedComponent::PrebuiltType(_)
                        | HydratedComponent::PreProcessingMiddleware(_)
                        | HydratedComponent::RequestHandler(_) => {}
                    }
                    input_types
                };

                for (input_index, input) in input_types.into_iter().enumerate() {
                    let Some(input) = input.as_ref() else {
                        continue;
                    };
                    // TODO: do we need this?
                    if let Some(id) = framework_items_db.get_id(input) {
                        if let Lifecycle::RequestScoped = framework_items_db.lifecycle(id) {
                            continue;
                        }
                    }

                    if let Some((input_component_id, mode)) = self.get_or_try_bind(
                        scope_id,
                        input,
                        component_db,
                        computation_db,
                        framework_items_db,
                    ) {
                        if ConsumptionMode::ExclusiveBorrow == mode {
                            let lifecycle = component_db.lifecycle(input_component_id);
                            match lifecycle {
                                Lifecycle::Singleton => {
                                    if let Some(user_component_id) =
                                        component_db.user_component_id(component_id)
                                    {
                                        Self::mut_ref_to_singleton(
                                            user_component_id,
                                            component_db.user_component_db(),
                                            input,
                                            input_index,
                                            package_graph,
                                            krate_collection,
                                            computation_db,
                                            diagnostics,
                                        )
                                    } else {
                                        tracing::warn!(
                                            "&mut singleton input ({:?}) for component {:?}, but the component is not a user component.",
                                            input,
                                            component_id
                                        );
                                    };
                                }
                                Lifecycle::RequestScoped => {
                                    let cloning_strategy =
                                        component_db.cloning_strategy(input_component_id);
                                    if cloning_strategy == CloningStrategy::CloneIfNecessary {
                                        if let Some(user_component_id) =
                                            component_db.user_component_id(component_id)
                                        {
                                            Self::mut_ref_to_clonable_request_scoped(
                                                user_component_id,
                                                component_db.user_component_db(),
                                                input,
                                                input_index,
                                                package_graph,
                                                krate_collection,
                                                computation_db,
                                                diagnostics,
                                            )
                                        } else {
                                            tracing::warn!(
                                                "&mut clonable request-scoped input ({:?}) for component {:?}, but the component is not a user component.",
                                                input,
                                                component_id
                                            );
                                        };
                                    }
                                }
                                Lifecycle::Transient => {
                                    if let Some(user_component_id) =
                                        component_db.user_component_id(component_id)
                                    {
                                        Self::mut_ref_to_transient(
                                            user_component_id,
                                            component_db.user_component_db(),
                                            input,
                                            input_index,
                                            package_graph,
                                            krate_collection,
                                            computation_db,
                                            diagnostics,
                                        )
                                    } else {
                                        tracing::warn!(
                                            "&mut transient input ({:?}) for component {:?}, but the component is not a user component.",
                                            input,
                                            component_id
                                        );
                                    };
                                }
                            }
                        }
                        continue;
                    }
                    if let Some(user_component_id) = component_db.user_component_id(component_id) {
                        Self::missing_constructor(
                            user_component_id,
                            component_db.user_component_db(),
                            input,
                            input_index,
                            package_graph,
                            krate_collection,
                            computation_db,
                            diagnostics,
                        )
                    } else {
                        tracing::warn!(
                            "Missing constructor for input type {:?} of component {:?}, but the component is not a user component.",
                            input,
                            component_id
                        );
                    }
                }
            }

            // If we didn't add any new component IDs, we're done.
            // Otherwise, we need to determine the list of component IDs that we are yet to examine.
            let new_component_ids: Vec<_> = component_db
                .iter()
                .skip(n_component_ids)
                .map(|(id, _)| id)
                .collect();
            if new_component_ids.is_empty() {
                break;
            } else {
                n_component_ids += new_component_ids.len();
                component_ids = new_component_ids;
            }
        }
    }

    /// Pavex guarantees that there is at most one live instance for each singleton type.
    ///
    /// If there are multiple constructors for the same singleton type, we end up in an ambiguous
    /// situation: which one do we use?
    /// We can't use both, because that would mean that there are two live instances of the same
    /// singleton type, which is not allowed.
    ///
    /// Disambiguating the constructors is the responsibility of the user.
    ///
    /// This method checks that there is at most one constructor for each singleton type.
    /// If that's not the case, we report this ambiguity as an error to the user.
    fn verify_singleton_ambiguity(
        &self,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let mut singleton_type2component_ids = HashMap::new();
        for (scope_id, constructibles) in &self.scope_id2constructibles {
            for (type_, component_id) in constructibles.type2constructor_id.iter() {
                if component_db.lifecycle(*component_id) != Lifecycle::Singleton {
                    continue;
                }
                let component_ids = singleton_type2component_ids
                    .entry(type_.clone())
                    .or_insert_with(IndexSet::new);
                component_ids.insert((*scope_id, component_id));
            }
        }

        for (type_, component_ids) in singleton_type2component_ids {
            if component_ids.len() > 1 {
                let n_unique_constructors = component_ids
                    .iter()
                    .map(|(_, &component_id)| {
                        component_db.hydrated_component(component_id, computation_db)
                    })
                    .collect::<HashSet<_>>()
                    .len();
                let n_constructors = component_ids.len();

                // For each component, we create an AnnotatedSnippet that points to the
                // registration site for that constructor.
                let mut snippets = Vec::new();
                let mut source_code = None;
                'inner: for (_, component_id) in &component_ids {
                    let (source, source_span) =
                        component_db.registration_span(**component_id, package_graph, diagnostics);
                    let Some(source) = source else {
                        continue 'inner;
                    };
                    let Some(source_span) = source_span else {
                        continue 'inner;
                    };
                    if source_code.is_none() {
                        source_code = Some(source.clone());
                    }
                    let label =
                        source_span.labeled("A constructor was registered here".to_string());
                    let snippet = AnnotatedSnippet::new(source, label);
                    snippets.push(snippet);
                }
                let diagnostic = if n_unique_constructors > 1 {
                    let error = anyhow::anyhow!(
                        "You can't register multiple constructors for the same singleton type, `{type_:?}`.\n\
                        There must be at most one live instance for each singleton type. \
                        If you register multiple constructors, I don't know which one to use to build \
                        that unique instance!\n\
                        I have found {n_constructors} different constructors for `{type_:?}`:",
                    );
                    CompilerDiagnostic::builder(error)
                        .optional_source(source_code)
                        .additional_annotated_snippets(snippets.into_iter())
                        .help(format!(
                            "If you want a single instance of `{type_:?}`, remove \
                            constructors for `{type_:?}` until there is only one left.\n\
                            If you want different instances, consider creating separate newtypes \
                            that wrap a `{type_:?}`."
                        ))
                        .build()
                } else {
                    fn get_help_snippet(
                        type_: &ResolvedType,
                        common_ancestor_id: ScopeId,
                        scope_graph: &ScopeGraph,
                        package_graph: &PackageGraph,
                        diagnostics: &mut Vec<miette::Error>,
                    ) -> Option<HelpWithSnippet> {
                        let location = scope_graph.get_location(common_ancestor_id).unwrap();
                        let source = try_source!(location, package_graph, diagnostics)?;
                        let label = diagnostic::get_bp_new_span(&source, &location).labeled(
                            "Register your constructor against this blueprint".to_string(),
                        )?;
                        Some(HelpWithSnippet::new(
                            format!(
                                "If you want to share a single instance of `{type_:?}`, remove \
                                    constructors for `{type_:?}` until there is only one left. It should \
                                    be attached to a blueprint that is a parent of all the nested \
                                    ones that need to use it."
                            ),
                            AnnotatedSnippet::new(source, label),
                        ))
                    }

                    let common_ancestor_scope_id = component_db.scope_graph().find_common_ancestor(
                        component_ids
                            .iter()
                            .map(|(scope_id, _)| *scope_id)
                            .collect(),
                    );
                    let help_snippet = get_help_snippet(
                        &type_,
                        common_ancestor_scope_id,
                        component_db.scope_graph(),
                        package_graph,
                        diagnostics,
                    );
                    let error = anyhow::anyhow!(
                        "The constructor for a singleton must be registered once.\n\
                        You registered the same constructor for `{type_:?}` against {n_constructors} \
                        different nested blueprints.\n\
                        I don't know how to proceed: do you want to share the same singleton instance across \
                        all those nested blueprints, or do you want to create a new instance for each \
                        nested blueprint?",
                    );
                    CompilerDiagnostic::builder(error)
                        .optional_source(source_code)
                        .additional_annotated_snippets(snippets.into_iter())
                        .optional_help_with_snippet(help_snippet)
                        .help(format!(
                            "If you want different instances, consider creating separate newtypes \
                            that wrap a `{type_:?}`."
                        ))
                        .build()
                };
                diagnostics.push(diagnostic.into());
            }
        }
    }

    /// Singletons are built before the application starts, outside of the request-response cycle.
    ///
    /// Therefore they cannot depend on types which have a request-scoped lifecycle.
    /// It's the responsibility of this method to enforce this constraint.
    fn verify_lifecycle_of_singleton_dependencies(
        &self,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        for (component_id, _) in component_db.iter() {
            if component_db.lifecycle(component_id) != Lifecycle::Singleton {
                continue;
            }
            let component = component_db.hydrated_component(component_id, computation_db);
            let component_scope = component_db.scope_id(component_id);
            for input_type in component.input_types().iter() {
                if let Some((input_constructor_id, _)) =
                    self.get(component_scope, input_type, component_db.scope_graph())
                {
                    if component_db.lifecycle(input_constructor_id) == Lifecycle::RequestScoped {
                        Self::singleton_must_not_depend_on_request_scoped(
                            component_id,
                            input_constructor_id,
                            package_graph,
                            component_db,
                            computation_db,
                            diagnostics,
                        )
                    }
                }
            }
        }
    }

    /// Error observers must be infallible—this extends to their dependencies.  
    /// This method checks that no error observer depends on a fallible component,
    /// either directly or transitively.
    ///
    /// # Rationale
    ///
    /// If an error observer depends on a fallible component, we'll have to invoke
    /// that fallible constructor _before_ invoking the error observer.  
    /// If the fallible constructor fails, we'll have to invoke the error observer
    /// on the error, which will in turn invoke the fallible constructor again,
    /// resulting in an infinite loop.
    fn error_observers_cannot_depend_on_fallible_components(
        &self,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        'outer: for (error_observer_id, _) in component_db.iter() {
            let HydratedComponent::ErrorObserver(eo) =
                component_db.hydrated_component(error_observer_id, computation_db)
            else {
                continue;
            };
            let mut queue = eo
                .input_types()
                .iter()
                .enumerate()
                .filter_map(|(i, input)| {
                    if i == eo.error_input_index {
                        return None;
                    }
                    Some((input.to_owned(), IndexSet::<ResolvedType>::new()))
                })
                .collect_vec();
            'inner: while let Some((input, mut dependency_chain)) = queue.pop() {
                let Some((input_constructor_id, _)) = self.get(
                    component_db.scope_id(error_observer_id),
                    &input,
                    component_db.scope_graph(),
                ) else {
                    continue 'inner;
                };
                if component_db.lifecycle(input_constructor_id) == Lifecycle::Singleton {
                    continue 'inner;
                }
                let HydratedComponent::Constructor(c) =
                    component_db.hydrated_component(input_constructor_id, computation_db)
                else {
                    continue 'inner;
                };
                if let Computation::MatchResult(_) = c.0 {
                    let fallible_id = component_db.fallible_id(input_constructor_id);
                    dependency_chain.insert(input.clone());
                    Self::error_observers_must_be_infallible(
                        error_observer_id,
                        dependency_chain,
                        fallible_id,
                        package_graph,
                        component_db,
                        computation_db,
                        diagnostics,
                    );
                    continue 'outer;
                }
                if !dependency_chain.insert(input) {
                    // We've already seen this type in the dependency chain, so we have a cycle.
                    // Cycle errors are detected elsewhere, so we don't need to do anything here.
                    // We just break to avoid infinite loops.
                    continue 'inner;
                }
                for input in c.input_types().iter() {
                    queue.push((input.to_owned(), dependency_chain.clone()));
                }
            }
        }
    }
}

impl ConstructibleDb {
    /// Find the constructor for a given type in a given scope.
    ///
    /// If the type is not constructible in the given scope, we look for a constructor in the
    /// parent scope, and so on until we reach the root scope.
    /// If we reach the root scope and the type still doesn't have a constructor, we return `None`.
    pub(crate) fn get(
        &self,
        scope_id: ScopeId,
        type_: &ResolvedType,
        scope_graph: &ScopeGraph,
    ) -> Option<(ComponentId, ConsumptionMode)> {
        let mut fifo = VecDeque::with_capacity(1);
        fifo.push_back(scope_id);
        while let Some(scope_id) = fifo.pop_front() {
            if let Some(constructibles) = self.scope_id2constructibles.get(&scope_id) {
                if let Some(output) = constructibles.get(type_) {
                    return Some(output);
                }
            }
            fifo.extend(scope_id.direct_parent_ids(scope_graph));
        }
        None
    }

    /// Add a new constructible type to the database.
    pub(crate) fn insert(
        &mut self,
        component_id: ComponentId,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
    ) {
        let component = component_db.hydrated_component(component_id, computation_db);
        assert!(matches!(component, HydratedComponent::Constructor(_)));
        let scope_id = component_db.scope_id(component_id);
        let scope_constructibles = self
            .scope_id2constructibles
            .entry(scope_id)
            .or_insert_with(ConstructiblesInScope::new);
        let output = component.output_type();
        scope_constructibles.insert(output.unwrap().to_owned(), component_id);
    }

    /// Find the constructor for a given type in a given scope.
    ///
    /// If the type is not constructible in the given scope, we look for a constructor in the
    /// parent scope, and so on until we reach the root scope.
    /// If we reach the root scope and the type still doesn't have a constructor, we return `None`.
    ///
    /// [`Self::get_or_try_bind`], compared to [`Self::get`], goes one step further: it inspects
    /// templated types to see if they can be instantiated in such a way to build
    /// the type that we want to construct.
    /// If that's the case, we bind the generic constructor, add it to the database and return
    /// the id of the newly bound constructor.
    pub(crate) fn get_or_try_bind(
        &mut self,
        scope_id: ScopeId,
        type_: &ResolvedType,
        component_db: &mut ComponentDb,
        computation_db: &mut ComputationDb,
        framework_item_db: &FrameworkItemDb,
    ) -> Option<(ComponentId, ConsumptionMode)> {
        let mut fifo = VecDeque::with_capacity(1);
        fifo.push_back(scope_id);
        while let Some(scope_id) = fifo.pop_front() {
            if let Some(constructibles) = self.scope_id2constructibles.get_mut(&scope_id) {
                if let Some(output) = constructibles.get_or_try_bind(
                    type_,
                    component_db,
                    computation_db,
                    framework_item_db,
                ) {
                    return Some(output);
                }
            }
            fifo.extend(scope_id.direct_parent_ids(component_db.scope_graph()));
        }
        None
    }

    fn missing_constructor(
        user_component_id: UserComponentId,
        user_component_db: &UserComponentDb,
        unconstructible_type: &ResolvedType,
        unconstructible_type_index: usize,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        computation_db: &ComputationDb,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        fn get_definition_info(
            callable: &Callable,
            unconstructible_type_index: usize,
            package_graph: &PackageGraph,
            krate_collection: &CrateCollection,
        ) -> Option<AnnotatedSnippet> {
            let def = CallableDefinition::compute(callable, krate_collection, package_graph)?;
            let input = &def.sig.inputs[unconstructible_type_index];
            let label = def.convert_local_span(input.span()).labeled(
                "I don't know how to construct an instance of this input parameter".into(),
            );
            Some(AnnotatedSnippet::new(def.named_source(), label))
        }

        let component_kind = user_component_db[user_component_id].callable_type();
        let location = user_component_db.get_location(user_component_id);
        let source = try_source!(location, package_graph, diagnostics);
        let label = source
            .as_ref()
            .map(|source| {
                diagnostic::get_f_macro_invocation_span(&source, location)
                    .labeled(format!("The {component_kind} was registered here"))
            })
            .flatten();

        let callable = &computation_db[user_component_id];
        let e = anyhow::anyhow!(
            "I can't find a constructor for `{unconstructible_type:?}`.\n\
            I need an instance of `{unconstructible_type:?}` to invoke your {component_kind}, `{}`.",
            callable.path
        );
        let definition_info = get_definition_info(
            callable,
            unconstructible_type_index,
            package_graph,
            krate_collection,
        );
        let diagnostic = CompilerDiagnostic::builder(e)
            .optional_source(source)
            .optional_label(label)
            .optional_additional_annotated_snippet(definition_info)
            .help_with_snippet(HelpWithSnippet::new(
                format!("Register a constructor for `{unconstructible_type:?}`."),
                AnnotatedSnippet::empty(),
            ))
            .help(format!(
                "Alternatively, use `Blueprint::prebuilt` to add a new input parameter of type `{unconstructible_type:?}` \
                to the (generated) `build_application_state`."
            ))
            .build();
        diagnostics.push(diagnostic.into());
    }

    fn mut_ref_to_singleton(
        user_component_id: UserComponentId,
        user_component_db: &UserComponentDb,
        singleton_input_type: &ResolvedType,
        singleton_input_index: usize,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        computation_db: &ComputationDb,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        fn get_snippet(
            callable: &Callable,
            krate_collection: &CrateCollection,
            package_graph: &PackageGraph,
            mut_ref_input_index: usize,
        ) -> Option<AnnotatedSnippet> {
            let def = CallableDefinition::compute(callable, krate_collection, package_graph)?;
            let input = &def.sig.inputs[mut_ref_input_index];
            let label = def
                .convert_local_span(input.span())
                .labeled("The &mut singleton".into());
            Some(AnnotatedSnippet::new(def.named_source(), label))
        }

        let component_kind = user_component_db[user_component_id].callable_type();
        let callable = &computation_db[user_component_id];
        let location = user_component_db.get_location(user_component_id);
        let source = try_source!(location, package_graph, diagnostics);
        let label = source
            .as_ref()
            .map(|source| {
                diagnostic::get_f_macro_invocation_span(&source, location)
                    .labeled(format!("The {component_kind} was registered here"))
            })
            .flatten();

        let definition_snippet = get_snippet(
            &computation_db[user_component_id],
            krate_collection,
            package_graph,
            singleton_input_index,
        );
        let error = anyhow::anyhow!("You can't inject a mutable reference to a singleton (`{singleton_input_type:?}`) as an input parameter to `{}`.", callable.path);
        let diagnostic = CompilerDiagnostic::builder(error)
            .optional_source(source)
            .optional_label(label)
            .optional_additional_annotated_snippet(definition_snippet)
            .help(
                "Singletons can only be taken via a shared reference (`&`) or by value (if cloneable). \
                If you absolutely need to mutate a singleton, consider internal mutability (e.g. `Arc<Mutex<..>>`).".into()
            )
            .build();
        diagnostics.push(diagnostic.into());
    }

    fn mut_ref_to_transient(
        user_component_id: UserComponentId,
        user_component_db: &UserComponentDb,
        transient_input_type: &ResolvedType,
        transient_input_index: usize,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        computation_db: &ComputationDb,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        fn get_snippet(
            callable: &Callable,
            krate_collection: &CrateCollection,
            package_graph: &PackageGraph,
            mut_ref_input_index: usize,
        ) -> Option<AnnotatedSnippet> {
            let def = CallableDefinition::compute(callable, krate_collection, package_graph)?;
            let input = &def.sig.inputs[mut_ref_input_index];
            let label = def
                .convert_local_span(input.span())
                .labeled("The &mut transient".into());
            Some(AnnotatedSnippet::new(def.named_source(), label))
        }

        let component_kind = user_component_db[user_component_id].callable_type();
        let callable = &computation_db[user_component_id];
        let location = user_component_db.get_location(user_component_id);
        let source = try_source!(location, package_graph, diagnostics);
        let label = source
            .as_ref()
            .map(|source| {
                diagnostic::get_f_macro_invocation_span(&source, location)
                    .labeled(format!("The {component_kind} was registered here"))
            })
            .flatten();

        let definition_snippet = get_snippet(
            &computation_db[user_component_id],
            krate_collection,
            package_graph,
            transient_input_index,
        );
        let error = anyhow::anyhow!("You can't inject a mutable reference to a transient type (`{transient_input_type:?}`) as an input parameter to `{}`.\n\
        Transient constructors are invoked every time their output is needed—instances of transient types are never reused. \
        The result of any mutation would be immediately discarded.", callable.path);
        let diagnostic = CompilerDiagnostic::builder(error)
            .optional_source(source)
            .optional_label(label)
            .optional_additional_annotated_snippet(definition_snippet)
            .help("Take the type by value, or use a `&` reference.".into())
            .build();
        diagnostics.push(diagnostic.into());
    }

    fn mut_ref_to_clonable_request_scoped(
        user_component_id: UserComponentId,
        user_component_db: &UserComponentDb,
        scoped_input_type: &ResolvedType,
        scoped_input_index: usize,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        computation_db: &ComputationDb,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        fn get_snippet(
            callable: &Callable,
            krate_collection: &CrateCollection,
            package_graph: &PackageGraph,
            mut_ref_input_index: usize,
        ) -> Option<AnnotatedSnippet> {
            let def = CallableDefinition::compute(callable, krate_collection, package_graph)?;
            let input = &def.sig.inputs[mut_ref_input_index];
            let label = def
                .convert_local_span(input.span())
                .labeled("The &mut reference".into());
            Some(AnnotatedSnippet::new(def.named_source(), label))
        }

        let component_kind = user_component_db[user_component_id].callable_type();
        let callable = &computation_db[user_component_id];
        let location = user_component_db.get_location(user_component_id);
        let source = try_source!(location, package_graph, diagnostics);
        let label = source
            .as_ref()
            .map(|source| {
                diagnostic::get_f_macro_invocation_span(&source, location)
                    .labeled(format!("The {component_kind} was registered here"))
            })
            .flatten();

        let definition_snippet = get_snippet(
            &computation_db[user_component_id],
            krate_collection,
            package_graph,
            scoped_input_index,
        );
        let ResolvedType::Reference(ref_) = scoped_input_type else {
            unreachable!()
        };
        let error = anyhow::anyhow!(
            "You can't inject `{scoped_input_type:?}` as an input parameter to `{}`, \
        since `{scoped_input_type:?}` has been marked `CloneIfNecessary`.\n\
        Reasoning about mutations becomes impossible if Pavex can't guarantee that all mutations \
        will affect *the same* instance of `{:?}`.",
            callable.path,
            ref_.inner
        );
        let diagnostic = CompilerDiagnostic::builder(error)
            .optional_source(source)
            .optional_label(label)
            .optional_additional_annotated_snippet(definition_snippet)
            .help(format!(
                "Change `{:?}`'s cloning strategy to `NeverClone`.",
                ref_.inner
            ))
            .build();
        diagnostics.push(diagnostic.into());
    }

    fn singleton_must_not_depend_on_request_scoped(
        singleton_id: ComponentId,
        dependency_id: ComponentId,
        package_graph: &PackageGraph,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let singleton_type = component_db
            .hydrated_component(singleton_id, computation_db)
            .output_type()
            .cloned()
            .unwrap();
        let dependency_type = component_db
            .hydrated_component(dependency_id, computation_db)
            .output_type()
            .cloned()
            .unwrap();
        let dependency_lifecycle = component_db.lifecycle(dependency_id);

        let e = anyhow::anyhow!(
            "Singletons can't depend on request-scoped components.\n\
            They are constructed before the application starts, outside of the request-response lifecycle.\n\
            But your singleton `{singleton_type:?}` depends on `{dependency_type:?}`, which has a {dependency_lifecycle} lifecycle.",
        );

        let (source, source_span) =
            component_db.registration_span(singleton_id, package_graph, diagnostics);
        let mut diagnostic_builder = CompilerDiagnostic::builder(e)
            .optional_source(source)
            .optional_label(source_span.labeled("The singleton was registered here".into()));

        if let (Some(source), source_span) =
            component_db.registration_span(dependency_id, package_graph, diagnostics)
        {
            diagnostic_builder =
                diagnostic_builder.additional_annotated_snippet(AnnotatedSnippet::new_optional(
                    source,
                    source_span.labeled(format!(
                        "The {dependency_lifecycle} dependency was registered here"
                    )),
                ));
        }
        diagnostics.push(diagnostic_builder.build().into());
    }

    fn error_observers_must_be_infallible(
        error_observer_id: ComponentId,
        dependency_chain: IndexSet<ResolvedType>,
        fallible_id: ComponentId,
        package_graph: &PackageGraph,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let HydratedComponent::ErrorObserver(error_observer) =
            component_db.hydrated_component(error_observer_id, computation_db)
        else {
            unreachable!()
        };
        let HydratedComponent::Constructor(fallible_constructor) =
            component_db.hydrated_component(fallible_id, computation_db)
        else {
            unreachable!()
        };
        let Computation::Callable(c) = &fallible_constructor.0 else {
            unreachable!()
        };
        let fallible_constructor_path = &c.path;

        let mut err_msg = String::new();
        for (i, type_) in dependency_chain.iter().enumerate() {
            use std::fmt::Write as _;
            if i != 0 {
                write!(err_msg, ", which depends on").unwrap();
            }
            write!(err_msg, " `{type_:?}`").unwrap();
        }

        let e = anyhow::anyhow!(
            "Error observers can't depend on a type with a fallible constructor, either directly or transitively.\n\
            `{}` violates this constraints! \
            It depends on{err_msg}, which is built with `{fallible_constructor_path}`, a fallible constructor.",
            error_observer.callable.path,
        );
        let (source, label) =
            component_db.registration_span(error_observer_id, package_graph, diagnostics);
        let diagnostic_builder = CompilerDiagnostic::builder(e)
            .optional_source(source)
            .optional_label(label.labeled("The error observer was registered here".into()));
        diagnostics.push(diagnostic_builder.build().into());
    }
}

/// The set of constructibles that have been registered in a given scope.
///
/// Be careful! This is not the set of all types that can be constructed in the given scope!
/// That's a much larger set, because it includes all types that can be constructed in this
/// scope as well as any of its parent scopes.
struct ConstructiblesInScope {
    type2constructor_id: HashMap<ResolvedType, ComponentId>,
    /// Every time we encounter a constructible type that contains an unassigned generic type
    /// (e.g. `T` in `Vec<T>` instead of `u8` in `Vec<u8>`), we store it here.
    ///
    /// This enables us to quickly determine if there might be a constructor for a given concrete
    /// type.
    /// For example, if you have a `Vec<u8>`, you first look in `type2constructor_id` to see if
    /// there is a constructor that returns `Vec<u8>`. If there isn't, you look in
    /// `generic_base_types` to see if there is a constructor that returns `Vec<T>`.
    ///
    /// Specialization, in a nutshell!
    templated_constructors: IndexSet<ResolvedType>,
}

impl ConstructiblesInScope {
    /// Create a new, empty set of constructibles.
    fn new() -> Self {
        Self {
            type2constructor_id: HashMap::new(),
            templated_constructors: IndexSet::new(),
        }
    }

    /// Retrieve the constructor for a given type, if it exists.
    fn get(&self, type_: &ResolvedType) -> Option<(ComponentId, ConsumptionMode)> {
        if let Some(constructor_id) = self.type2constructor_id.get(type_).copied() {
            return Some((constructor_id, ConsumptionMode::Move));
        }

        match type_ {
            ResolvedType::Reference(ref_) if !ref_.lifetime.is_static() => {
                if let Some(constructor_id) = self.type2constructor_id.get(&ref_.inner).copied() {
                    return Some((
                        constructor_id,
                        if ref_.is_mutable {
                            ConsumptionMode::ExclusiveBorrow
                        } else {
                            ConsumptionMode::SharedBorrow
                        },
                    ));
                }
            }
            _ => {}
        }

        None
    }

    /// Retrieve the constructor for a given type, if it exists.
    ///
    /// If it doesn't exist, check the templated constructors to see if there is a constructor
    /// that can be specialized to construct the given type.
    fn get_or_try_bind(
        &mut self,
        type_: &ResolvedType,
        component_db: &mut ComponentDb,
        computation_db: &mut ComputationDb,
        framework_item_db: &FrameworkItemDb,
    ) -> Option<(ComponentId, ConsumptionMode)> {
        if let Some(output) = self.get(type_) {
            return Some(output);
        }
        for templated_constructible_type in &self.templated_constructors {
            if let Some(bindings) = templated_constructible_type.is_a_template_for(type_) {
                let template = templated_constructible_type.clone();
                let (templated_component_id, _) = self.get(&template).unwrap();
                self.bind_and_register_constructor(
                    templated_component_id,
                    component_db,
                    computation_db,
                    framework_item_db,
                    &bindings,
                );
                let bound = self.get(type_);
                assert!(bound.is_some(), "I used {} as a templated constructor to build {} but the binding process didn't succeed as expected.\nBindings:\n{}", 
                    template.display_for_error(),
                    type_.display_for_error(),
                    bindings.into_iter().map(|(k, v)| format!("- {k} -> {}", v.display_for_error())).collect::<Vec<_>>().join("\n")
                );
                return bound;
            }
        }

        match type_ {
            ResolvedType::Reference(ref_) if !ref_.lifetime.is_static() => {
                let (component_id, _) = self.get_or_try_bind(
                    &ref_.inner,
                    component_db,
                    computation_db,
                    framework_item_db,
                )?;
                let lifecycle = component_db.lifecycle(component_id);
                if ref_.is_mutable {
                    match lifecycle {
                        Lifecycle::Singleton => {
                            // TODO: emit error diagnostic
                            panic!("You can't take a mutable reference to a singleton")
                        }
                        Lifecycle::Transient => {
                            // TODO: emit warning diagnostic.
                        }
                        Lifecycle::RequestScoped => {}
                    }
                }
                Some((
                    component_id,
                    if ref_.is_mutable {
                        ConsumptionMode::ExclusiveBorrow
                    } else {
                        ConsumptionMode::SharedBorrow
                    },
                ))
            }
            _ => None,
        }
    }

    /// Register a type and its constructor.
    fn insert(&mut self, output: ResolvedType, component_id: ComponentId) {
        if output.is_a_template() {
            self.templated_constructors.insert(output.clone());
        }
        self.type2constructor_id.insert(output, component_id);
    }

    /// Specialize a templated constructor to a concrete type.
    ///
    /// The newly bound components are added to all the relevant databases.
    fn bind_and_register_constructor(
        &mut self,
        templated_component_id: ComponentId,
        component_db: &mut ComponentDb,
        computation_db: &mut ComputationDb,
        framework_item_db: &FrameworkItemDb,
        bindings: &HashMap<String, ResolvedType>,
    ) {
        let bound_component_id = component_db.bind_generic_type_parameters(
            templated_component_id,
            bindings,
            computation_db,
            framework_item_db,
        );

        let mut derived_component_ids = component_db.derived_component_ids(bound_component_id);
        derived_component_ids.push(bound_component_id);

        for derived_component_id in derived_component_ids {
            let component = component_db.hydrated_component(derived_component_id, computation_db);
            if let HydratedComponent::Constructor(c) = component {
                self.type2constructor_id
                    .insert(c.output_type().clone(), derived_component_id);
            }
        }
    }
}

impl std::fmt::Debug for ConstructiblesInScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Constructibles:")?;
        for (type_, component_id) in &self.type2constructor_id {
            writeln!(f, "- {} -> {:?}", type_.display_for_error(), component_id)?;
        }
        Ok(())
    }
}
