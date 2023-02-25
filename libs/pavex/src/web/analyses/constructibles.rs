use std::borrow::Cow;

use ahash::{HashMap, HashMapExt, HashSet};
use guppy::graph::PackageGraph;
use indexmap::IndexSet;
use miette::NamedSource;
use syn::spanned::Spanned;

use pavex_builder::Lifecycle;

use crate::diagnostic;
use crate::diagnostic::{
    convert_proc_macro_span, convert_rustdoc_span, read_source_file, AnnotatedSnippet,
    CompilerDiagnostic, LocationExt, SourceSpanExt,
};
use crate::language::{
    Callable, GenericArgument, NamedTypeGeneric, ResolvedPathType, ResolvedType, Slice, Tuple,
    TypeReference,
};
use crate::rustdoc::CrateCollection;
use crate::web::analyses::components::{ComponentDb, ComponentId, HydratedComponent};
use crate::web::analyses::computations::ComputationDb;
use crate::web::analyses::raw_identifiers::RawCallableIdentifiersDb;
use crate::web::analyses::user_components::{UserComponentDb, UserComponentId};
use crate::web::computation::Computation;

#[derive(Debug)]
pub(crate) struct ConstructibleDb {
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

impl ConstructibleDb {
    pub(crate) fn build(
        component_db: &mut ComponentDb,
        computation_db: &mut ComputationDb,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        user_component_db: &UserComponentDb,
        raw_identifiers_db: &RawCallableIdentifiersDb,
        request_scoped_framework_types: &HashSet<&ResolvedType>,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Self {
        let mut type2constructor_id = HashMap::new();
        let mut templated_constructors = IndexSet::new();
        for (component_id, component) in component_db.constructors(computation_db) {
            let output = component.output_type();
            type2constructor_id.insert(output.to_owned(), component_id);
            if can_be_specialized(&output) {
                templated_constructors.insert(output.to_owned());
            }
        }
        let mut self_ = Self {
            type2constructor_id,
            templated_constructors,
        };

        let component_ids = component_db.iter().map(|(id, _)| id).collect::<Vec<_>>();
        for component_id in component_ids {
            let resolved_component = component_db.hydrated_component(component_id, computation_db);
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

            'outer: for (input_index, input) in input_types.into_iter().enumerate() {
                let input = match input.as_ref() {
                    Some(i) => i,
                    None => {
                        continue;
                    }
                };
                if request_scoped_framework_types.contains(input) {
                    continue;
                }
                if self_.get(input).is_some() {
                    continue;
                }
                for templated_constructible_type in &self_.templated_constructors {
                    if let Some(bindings) =
                        can_be_specialized_to(input, templated_constructible_type)
                    {
                        specialize_and_register_constructor(
                            self_[templated_constructible_type],
                            component_db,
                            computation_db,
                            &mut self_,
                            &bindings,
                        );
                        continue 'outer;
                    }
                }
                if let Some(user_component_id) = component_db.user_component_id(component_id) {
                    ConstructibleDb::missing_constructor(
                        user_component_id,
                        user_component_db,
                        input,
                        input_index,
                        package_graph,
                        krate_collection,
                        raw_identifiers_db,
                        computation_db,
                        diagnostics,
                    )
                } else {
                    unreachable!()
                }
            }
        }

        self_
    }

    fn missing_constructor(
        user_component_id: UserComponentId,
        user_component_db: &UserComponentDb,
        unconstructible_type: &ResolvedType,
        unconstructible_type_index: usize,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        raw_identifiers_db: &RawCallableIdentifiersDb,
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
                    } else if let Ok(item) = syn::parse_str::<syn::ImplItemMethod>(span_contents) {
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
            .labeled("I do not know how to construct an instance of this input parameter".into());
            let source_path = definition_span.filename.to_str().unwrap();
            Some(AnnotatedSnippet::new(
                NamedSource::new(source_path, source_contents),
                label,
            ))
        }

        let user_component = &user_component_db[user_component_id];
        let callable = &computation_db[user_component_id];
        let raw_identifier_id = user_component.raw_callable_identifiers_id();
        let component_kind = user_component.callable_type();

        let location = raw_identifiers_db.get_location(raw_identifier_id);
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
                "I cannot invoke your {component_kind}, `{}`, because it needs an instance \
                of `{unconstructible_type:?}` as input, but I cannot find a constructor for that type.",
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

    pub(crate) fn get(&self, t: &ResolvedType) -> Option<ComponentId> {
        self.type2constructor_id.get(t).cloned()
    }
}

impl std::ops::Index<&ResolvedType> for ConstructibleDb {
    type Output = ComponentId;

    fn index(&self, index: &ResolvedType) -> &Self::Output {
        &self.type2constructor_id[index]
    }
}

fn specialize_and_register_constructor(
    templated_component_id: ComponentId,
    component_db: &mut ComponentDb,
    computation_db: &mut ComputationDb,
    constructible_db: &mut ConstructibleDb,
    bindings: &HashMap<NamedTypeGeneric, ResolvedType>,
) {
    let lifecycle = component_db
        .lifecycle(templated_component_id)
        .unwrap()
        .to_owned();
    let templated_component = component_db
        .hydrated_component(templated_component_id, computation_db)
        .into_owned();
    let HydratedComponent::Constructor(templated_constructor) = templated_component else { unreachable!() };
    let specialized_output_type =
        bind_generic_type_parameters(templated_constructor.output_type(), &bindings);
    match &templated_constructor.0 {
        Computation::Callable(c) => {
            let specialized_callable = Callable {
                output: Some(specialized_output_type.clone()),
                ..c.clone().into_owned()
            };
            let computation = Computation::Callable(Cow::Owned(specialized_callable));
            let computation_id = computation_db.get_or_intern(computation);
            let specialized_component_id = component_db
                .get_or_intern_constructor(computation_id, lifecycle, computation_db)
                .unwrap();
            constructible_db
                .type2constructor_id
                .insert(specialized_output_type, specialized_component_id);
            for derived_component_id in component_db.derived_component_ids(specialized_component_id)
            {
                if let HydratedComponent::Constructor(c) =
                    component_db.hydrated_component(derived_component_id, computation_db)
                {
                    constructible_db
                        .type2constructor_id
                        .insert(c.output_type().clone(), derived_component_id);
                }
            }
        }
        Computation::MatchResult(_) => {
            let fallible_constructor_id = component_db.fallible_id(templated_component_id);
            specialize_and_register_constructor(
                fallible_constructor_id,
                component_db,
                computation_db,
                constructible_db,
                bindings,
            );
        }
        Computation::BorrowSharedReference(_) => {
            let owned_constructor_id = component_db.owned_id(templated_component_id);
            specialize_and_register_constructor(
                owned_constructor_id,
                component_db,
                computation_db,
                constructible_db,
                bindings,
            );
        }
    }
}

/// Replace unassigned generic type parameters in `templated_type` with the concrete generic type
/// parameters defined in `bindings`.
///
/// This function can also be used to _partially_ bind the unassigned generic type parameters in
/// `t`. You are not required to bind all of them.
fn bind_generic_type_parameters(
    t: &ResolvedType,
    bindings: &HashMap<NamedTypeGeneric, ResolvedType>,
) -> ResolvedType {
    match t {
        ResolvedType::ResolvedPath(t) => {
            let mut bound_generics = Vec::with_capacity(t.generic_arguments.len());
            for generic in &t.generic_arguments {
                let bound_generic = match generic {
                    GenericArgument::UnassignedTypeParameter(name) => {
                        if let Some(bound_type) = bindings.get(name) {
                            GenericArgument::AssignedTypeParameter(bound_type.clone())
                        } else {
                            generic.to_owned()
                        }
                    }
                    GenericArgument::AssignedTypeParameter(t) => {
                        GenericArgument::AssignedTypeParameter(bind_generic_type_parameters(
                            t, bindings,
                        ))
                    }
                    GenericArgument::Lifetime(_) => generic.to_owned(),
                };
                bound_generics.push(bound_generic);
            }
            ResolvedType::ResolvedPath(ResolvedPathType {
                package_id: t.package_id.clone(),
                // Should we set this to `None`?
                rustdoc_id: t.rustdoc_id.clone(),
                base_type: t.base_type.clone(),
                generic_arguments: bound_generics,
            })
        }
        ResolvedType::Reference(r) => ResolvedType::Reference(TypeReference {
            is_mutable: r.is_mutable,
            inner: Box::new(bind_generic_type_parameters(&r.inner, bindings)),
            is_static: r.is_static,
        }),
        ResolvedType::Tuple(t) => {
            let mut bound_elements = Vec::with_capacity(t.elements.len());
            for inner in &t.elements {
                bound_elements.push(bind_generic_type_parameters(inner, bindings));
            }
            ResolvedType::Tuple(Tuple {
                elements: bound_elements,
            })
        }
        ResolvedType::ScalarPrimitive(s) => ResolvedType::ScalarPrimitive(s.clone()),
        ResolvedType::Slice(s) => ResolvedType::Slice(Slice {
            element_type: Box::new(bind_generic_type_parameters(&s.element_type, bindings)),
        }),
    }
}

/// Check if a type can be considered a "specialization" of another, with respect to their generic parameters.
///
/// I.e. if by replacing the unassigned generic type parameters of `templated_type` with the
/// concrete generic type parameters of `concrete_type`, `templated_type` would be equal to `concrete_type`.
///
/// If possible, this function will return a map associating each unassigned generic parameter
/// in `templated_type` with the type it must be set to in order to match `concrete_type`.
/// If impossible, this function will return `None`.
#[tracing::instrument(level = "trace", ret)]
fn can_be_specialized_to(
    concrete_type: &ResolvedType,
    templated_type: &ResolvedType,
) -> Option<HashMap<NamedTypeGeneric, ResolvedType>> {
    let mut bindings = HashMap::new();
    if _can_be_specialized_to(concrete_type, templated_type, &mut bindings) {
        Some(bindings)
    } else {
        None
    }
}

#[tracing::instrument(level = "trace", ret)]
fn _can_be_specialized_to(
    concrete_type: &ResolvedType,
    templated_type: &ResolvedType,
    bindings: &mut HashMap<NamedTypeGeneric, ResolvedType>,
) -> bool {
    if concrete_type == templated_type {
        return true;
    }
    use ResolvedType::*;
    match (concrete_type, templated_type) {
        (ResolvedPath(concrete_path), ResolvedPath(templated_path)) => {
            _can_be_specialized_to_for_resolved_path_types(concrete_path, templated_path, bindings)
        }
        (Slice(concrete_slice), Slice(templated_slice)) => _can_be_specialized_to(
            &concrete_slice.element_type,
            &templated_slice.element_type,
            bindings,
        ),
        (Reference(concrete_reference), Reference(templated_reference)) => _can_be_specialized_to(
            &concrete_reference.inner,
            &templated_reference.inner,
            bindings,
        ),
        (Tuple(concrete_tuple), Tuple(templated_tuple)) => {
            if concrete_tuple.elements.len() != templated_tuple.elements.len() {
                return false;
            }
            concrete_tuple
                .elements
                .iter()
                .zip(templated_tuple.elements.iter())
                .all(|(concrete_type, templated_type)| {
                    _can_be_specialized_to(concrete_type, templated_type, bindings)
                })
        }
        (ScalarPrimitive(concrete_primitive), ScalarPrimitive(templated_primitive)) => {
            concrete_primitive == templated_primitive
        }
        (_, _) => false,
    }
}

fn _can_be_specialized_to_for_resolved_path_types(
    concrete_type: &ResolvedPathType,
    templated_type: &ResolvedPathType,
    bindings: &mut HashMap<NamedTypeGeneric, ResolvedType>,
) -> bool {
    // We destructure ALL fields to make sure that the compiler reminds us to update
    // this function if we add new fields to `ResolvedPathType`.
    let ResolvedPathType {
        package_id: concrete_package_id,
        rustdoc_id: _,
        base_type: concrete_base_type,
        generic_arguments: concrete_generic_arguments,
    } = concrete_type;
    let ResolvedPathType {
        package_id: templated_package_id,
        rustdoc_id: _,
        base_type: templated_base_type,
        generic_arguments: templated_generic_arguments,
    } = templated_type;
    if concrete_package_id != templated_package_id
        || concrete_base_type != templated_base_type
        || concrete_generic_arguments.len() != templated_generic_arguments.len()
    {
        return false;
    }
    for (concrete_arg, templated_arg) in concrete_generic_arguments
        .iter()
        .zip(templated_generic_arguments.iter())
    {
        use GenericArgument::*;
        match (concrete_arg, templated_arg) {
            (
                AssignedTypeParameter(concrete_arg_type),
                AssignedTypeParameter(templated_arg_type),
            ) => {
                if !_can_be_specialized_to(concrete_arg_type, templated_arg_type, bindings) {
                    return false;
                }
            }
            (AssignedTypeParameter(assigned), UnassignedTypeParameter(unassigned)) => {
                // The unassigned type parameter can be assigned to the concrete type
                // we expect, so it is a specialization.
                let previous_assignment = bindings.insert(unassigned.clone(), assigned.clone());
                if let Some(previous_assignment) = previous_assignment {
                    if &previous_assignment != assigned {
                        tracing::trace!(
                            "Type parameter `{:?}` was already assigned to `{:?}` but is now being assigned to `{:?}`",
                            unassigned,
                            previous_assignment,
                            assigned
                        );
                        return false;
                    }
                }
            }
            (Lifetime(_), Lifetime(_)) => {
                // Lifetimes are not relevant for specialization (yet).
            }
            (UnassignedTypeParameter(unassigned), _) => {
                // You are not allowed to specialize a type with an unassigned type parameter.
                unreachable!("Unassigned type parameter (`{:?}`) in the 'concrete' type (`{:?}`) when checking for specialization", unassigned, concrete_type);
            }
            (AssignedTypeParameter(_), Lifetime(_))
            | (Lifetime(_), UnassignedTypeParameter(_))
            | (Lifetime(_), AssignedTypeParameter(_)) => {
                return false;
            }
        }
    }
    true
}

/// Check if a type can be "specialized" - i.e. if it has any unassigned generic type parameters.
#[tracing::instrument(level = "trace", ret)]
fn can_be_specialized(t: &ResolvedType) -> bool {
    match t {
        ResolvedType::ResolvedPath(path) => path.generic_arguments.iter().any(|arg| match arg {
            GenericArgument::UnassignedTypeParameter(_) => true,
            _ => false,
        }),
        ResolvedType::Reference(r) => can_be_specialized(&r.inner),
        ResolvedType::Tuple(t) => t.elements.iter().any(|t| can_be_specialized(t)),
        ResolvedType::ScalarPrimitive(_) => false,
        ResolvedType::Slice(s) => can_be_specialized(&s.element_type),
    }
}
