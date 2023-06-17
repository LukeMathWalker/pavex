use std::collections::VecDeque;

use ahash::{HashMap, HashMapExt, HashSet};
use guppy::graph::PackageGraph;
use indexmap::{IndexMap, IndexSet};
use miette::{NamedSource, SourceSpan};
use syn::spanned::Spanned;

use pavex::blueprint::constructor::Lifecycle;

use crate::compiler::analyses::components::{
    ComponentDb, ComponentId, ConsumptionMode, HydratedComponent,
};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::user_components::{
    ScopeGraph, ScopeId, UserComponentDb, UserComponentId,
};
use crate::diagnostic::{self, ParsedSourceFile};
use crate::diagnostic::{
    convert_proc_macro_span, convert_rustdoc_span, read_source_file, AnnotatedSnippet,
    CompilerDiagnostic, HelpWithSnippet, LocationExt, SourceSpanExt,
};
use crate::language::{Callable, ResolvedType};
use crate::rustdoc::CrateCollection;

use super::framework_items::FrameworkItemDb;

#[derive(Debug)]
/// The set of types that can be injected into request handlers, error handlers and (other) constructors.
pub(crate) struct ConstructibleDb {
    scope_id2constructibles: IndexMap<ScopeId, ConstructiblesInScope>,
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

        self_
    }

    fn _build(component_db: &mut ComponentDb, computation_db: &mut ComputationDb) -> Self {
        let mut scope_id2constructibles = IndexMap::new();
        for (component_id, component) in component_db.constructors(computation_db) {
            let scope_id = component_db.scope_id(component_id);
            let scope_constructibles = scope_id2constructibles
                .entry(scope_id)
                .or_insert_with(ConstructiblesInScope::new);
            let output = component.output_type();
            scope_constructibles.insert(output.to_owned(), component_id);
        }
        Self {
            scope_id2constructibles,
        }
    }

    /// Check if any component is asking for a type as input parameter for which there is no
    /// constructor.
    ///
    /// This check skips singletons, since they are going to be provided by the user as part
    /// of the application state if there have no registered constructor.
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
                // We don't support dependency injection for transformers (yet).
                if let HydratedComponent::Transformer(_) = &resolved_component {
                    continue;
                }

                if let HydratedComponent::Constructor(_) = &resolved_component {
                    let lifecycle = component_db.lifecycle(component_id).unwrap();
                    if lifecycle == &Lifecycle::Singleton {
                        continue;
                    }
                }

                let input_types = {
                    let mut input_types: Vec<Option<ResolvedType>> = resolved_component
                        .input_types()
                        .iter()
                        .map(|i| Some(i.to_owned()))
                        .collect();
                    // Errors happen, they are not "constructed" (we use a transformer instead).
                    // Therefore we skip the error input type for error handlers.
                    if let HydratedComponent::ErrorHandler(e) = &resolved_component {
                        input_types[e.error_input_index] = None;
                    }
                    input_types
                };

                for (input_index, input) in input_types.into_iter().enumerate() {
                    let input = match input.as_ref() {
                        Some(i) => i,
                        None => {
                            continue;
                        }
                    };
                    if let Some(id) = framework_items_db.get_id(input) {
                        if let Lifecycle::RequestScoped = framework_items_db.lifecycle(id) {
                            continue;
                        }
                    }
                    if self
                        .get_or_try_bind(scope_id, input, component_db, computation_db)
                        .is_some()
                    {
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
                let lifecycle = component_db.lifecycle(*component_id).unwrap();
                if lifecycle != &Lifecycle::Singleton {
                    continue;
                }
                let component_ids = singleton_type2component_ids
                    .entry(type_.clone())
                    .or_insert_with(IndexSet::new);
                component_ids.insert((*scope_id, component_id));
            }
        }

        'outer: for (type_, component_ids) in singleton_type2component_ids {
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
                    let Some(user_component_id) = component_db.user_component_id(**component_id) else {
                        continue 'inner;
                    };
                    let location = component_db
                        .user_component_db()
                        .get_location(user_component_id);
                    let source = match location.source_file(package_graph) {
                        Ok(s) => s,
                        Err(e) => {
                            diagnostics.push(e.into());
                            continue 'inner;
                        }
                    };
                    if source_code.is_none() {
                        source_code = Some(source.clone());
                    }
                    let label = diagnostic::get_f_macro_invocation_span(&source, location)
                        .map(|s| s.labeled("A constructor was registered here".to_string()));
                    if let Some(label) = label {
                        let snippet = AnnotatedSnippet::new(source, label);
                        snippets.push(snippet);
                    }
                }
                let Some(source_code) = source_code else {
                    continue 'outer;
                };
                let diagnostic = if n_unique_constructors > 1 {
                    let error = anyhow::anyhow!(
                        "You can't register multiple constructors for the same singleton type, `{type_:?}`.\n\
                        There must be at most one live instance for each singleton type. \
                        If you register multiple constructors, I don't know which one to use to build \
                        that unique instance!\n\
                        I have found {n_constructors} different constructors for `{type_:?}`:",
                    );
                    CompilerDiagnostic::builder(source_code, error)
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
                        let source = match location.source_file(package_graph) {
                            Ok(s) => s,
                            Err(e) => {
                                diagnostics.push(e.into());
                                return None;
                            }
                        };
                        let label = diagnostic::get_bp_new_span(&source, &location).map(|s| {
                            s.labeled(
                                "Register your constructor against this blueprint".to_string(),
                            )
                        });
                        if let Some(label) = label {
                            Some(HelpWithSnippet::new(
                                format!(
                                    "If you want to share a single instance of `{type_:?}`, remove \
                                    constructors for `{type_:?}` until there is only one left. It should \
                                    be attached to a blueprint that is a parent of all the nested \
                                    ones that need to use it."
                                ),
                                AnnotatedSnippet::new(source, label),
                            ))
                        } else {
                            None
                        }
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
                    CompilerDiagnostic::builder(source_code, error)
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
    /// Therefore they cannot depend on types which have a shorter lifecycle—i.e. request-scoped
    /// or transient.
    /// It's the responsibility of this method to enforce this constraint.
    fn verify_lifecycle_of_singleton_dependencies(
        &self,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        for (component_id, _) in component_db.iter() {
            if component_db.lifecycle(component_id) != Some(&Lifecycle::Singleton) {
                continue;
            }
            let component = component_db.hydrated_component(component_id, computation_db);
            let component_scope = component_db.scope_id(component_id);
            for input_type in component.input_types().iter() {
                if let Some((input_constructor_id, _)) =
                    self.get(component_scope, input_type, component_db.scope_graph())
                {
                    let input_lifecycle = component_db.lifecycle(input_constructor_id).unwrap();
                    if input_lifecycle != &Lifecycle::Singleton {
                        Self::singleton_must_depend_on_singletons(
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
    fn get_or_try_bind(
        &mut self,
        scope_id: ScopeId,
        type_: &ResolvedType,
        component_db: &mut ComponentDb,
        computation_db: &mut ComputationDb,
    ) -> Option<(ComponentId, ConsumptionMode)> {
        let mut fifo = VecDeque::with_capacity(1);
        fifo.push_back(scope_id);
        while let Some(scope_id) = fifo.pop_front() {
            if let Some(constructibles) = self.scope_id2constructibles.get_mut(&scope_id) {
                if let Some(output) =
                    constructibles.get_or_try_bind(type_, component_db, computation_db)
                {
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
            let (callable_type, _) = callable.path.find_rustdoc_items(krate_collection).ok()?;
            let callable_item = callable_type.item.item;
            let definition_span = callable_item.span.as_ref()?;
            let source_contents =
                read_source_file(&definition_span.filename, &package_graph.workspace()).ok()?;
            let span = convert_rustdoc_span(&source_contents, definition_span.to_owned());
            let span_contents = &source_contents[span.offset()..(span.offset() + span.len())];
            let input = match &callable_item.inner {
                rustdoc_types::ItemEnum::Function(_) => {
                    if let Ok(item) = syn::parse_str::<syn::ItemFn>(span_contents) {
                        let mut inputs = item.sig.inputs.iter();
                        inputs.nth(unconstructible_type_index).cloned()
                    } else if let Ok(item) = syn::parse_str::<syn::ImplItemFn>(span_contents) {
                        let mut inputs = item.sig.inputs.iter();
                        inputs.nth(unconstructible_type_index).cloned()
                    } else {
                        eprintln!("Could not parse as a function or method:\n{span_contents}");
                        return None;
                    }
                }
                _ => unreachable!(),
            }?;
            let s = convert_proc_macro_span(
                span_contents,
                match input {
                    syn::FnArg::Typed(typed) => typed.ty.span(),
                    syn::FnArg::Receiver(r) => r.span(),
                },
            );
            let label = miette::SourceSpan::new(
                // We must shift the offset forward because it's the
                // offset from the beginning of the file slice that
                // we deserialized, instead of the entire file
                (s.offset() + span.offset()).into(),
                s.len().into(),
            )
            .labeled("I don't know how to construct an instance of this input parameter".into());
            let source_path = definition_span.filename.to_str().unwrap();
            Some(AnnotatedSnippet::new(
                NamedSource::new(source_path, source_contents),
                label,
            ))
        }

        let user_component = &user_component_db[user_component_id];
        let callable = &computation_db[user_component_id];
        let component_kind = user_component.callable_type();
        let location = user_component_db.get_location(user_component_id);

        let source = match location.source_file(package_graph) {
            Ok(s) => s,
            Err(e) => {
                diagnostics.push(e.into());
                return;
            }
        };
        let label = diagnostic::get_f_macro_invocation_span(&source, location)
            .map(|s| s.labeled(format!("The {component_kind} was registered here")));
        let e = anyhow::anyhow!(
                "I can't invoke your {component_kind}, `{}`, because it needs an instance \
                of `{unconstructible_type:?}` as input, but I can't find a constructor for that type.",
                callable.path
            );
        let definition_info = get_definition_info(
            callable,
            unconstructible_type_index,
            package_graph,
            krate_collection,
        );
        let diagnostic = CompilerDiagnostic::builder(source, e)
            .optional_label(label)
            .optional_additional_annotated_snippet(definition_info)
            .help(format!(
                "Register a constructor for `{unconstructible_type:?}`"
            ))
            .build();
        diagnostics.push(diagnostic.into());
    }

    fn singleton_must_depend_on_singletons(
        singleton_id: ComponentId,
        dependency_id: ComponentId,
        package_graph: &PackageGraph,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        fn registration_span(
            component_id: ComponentId,
            package_graph: &PackageGraph,
            component_db: &ComponentDb,
            diagnostics: &mut Vec<miette::Error>,
        ) -> Option<(ParsedSourceFile, SourceSpan)> {
            let user_id = component_db.user_component_id(component_id)?;
            let user_component_db = component_db.user_component_db();
            let location = user_component_db.get_location(user_id);

            let source = match location.source_file(package_graph) {
                Ok(s) => s,
                Err(e) => {
                    diagnostics.push(e.into());
                    return None;
                }
            };
            let source_span = diagnostic::get_f_macro_invocation_span(&source, location)?;
            Some((source, source_span))
        }

        let singleton_type = component_db
            .hydrated_component(singleton_id, computation_db)
            .output_type()
            .to_owned();
        let dependency_type = component_db
            .hydrated_component(dependency_id, computation_db)
            .output_type()
            .to_owned();
        let dependency_lifecycle = component_db.lifecycle(dependency_id).unwrap();

        let e = anyhow::anyhow!(
            "Singletons can't depend on request-scoped or transient components.\n\
            They are constructed before the application starts, outside of the request-response lifecycle.\n\
            But your singleton `{singleton_type:?}` depends on `{dependency_type:?}`, which has a {dependency_lifecycle} lifecycle.",
        );
        let mut diagnostic_builder =
            match registration_span(singleton_id, package_graph, component_db, diagnostics) {
                Some((source, source_span)) => CompilerDiagnostic::builder(source, e)
                    .label(source_span.labeled("The singleton was registered here".into())),
                None => {
                    CompilerDiagnostic::builder(NamedSource::new("".to_string(), "".to_string()), e)
                }
            };

        if let Some((source, source_span)) =
            registration_span(dependency_id, package_graph, component_db, diagnostics)
        {
            diagnostic_builder =
                diagnostic_builder.additional_annotated_snippet(AnnotatedSnippet::new(
                    source,
                    source_span.labeled(format!(
                        "The {dependency_lifecycle} dependency was registered here"
                    )),
                ));
        }
        diagnostics.push(diagnostic_builder.build().into());
    }
}

#[derive(Debug)]
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
            ResolvedType::Reference(ref_) if !ref_.is_static && !ref_.is_mutable => {
                if let Some(constructor_id) = self.type2constructor_id.get(&ref_.inner).copied() {
                    return Some((constructor_id, ConsumptionMode::SharedBorrow));
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
    ) -> Option<(ComponentId, ConsumptionMode)> {
        if let Some(output) = self.get(type_) {
            return Some(output);
        }
        for templated_constructible_type in &self.templated_constructors {
            if let Some(bindings) = templated_constructible_type.is_a_template_for(type_) {
                let (templated_component_id, _) = self.get(templated_constructible_type).unwrap();
                self.bind_and_register_constructor(
                    templated_component_id,
                    component_db,
                    computation_db,
                    &bindings,
                );
                return self.get(type_);
            }
        }

        match type_ {
            ResolvedType::Reference(ref_) if !ref_.is_static && !ref_.is_mutable => {
                let (component_id, _) =
                    self.get_or_try_bind(&ref_.inner, component_db, computation_db)?;
                Some((component_id, ConsumptionMode::SharedBorrow))
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
        bindings: &HashMap<String, ResolvedType>,
    ) {
        let bound_component_id = component_db.bind_generic_type_parameters(
            templated_component_id,
            bindings,
            computation_db,
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
