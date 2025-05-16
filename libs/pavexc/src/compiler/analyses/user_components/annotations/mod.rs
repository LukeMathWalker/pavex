use std::{collections::BTreeMap, ops::Deref, sync::Arc};

mod diagnostic;
mod overlay;
mod queue;
mod registry;
mod sortable;

use diagnostic::{const_generics_are_not_supported, not_a_module, unknown_module_path};
pub use overlay::augment_from_annotation;
pub use registry::*;

use super::{
    ScopeId, UserComponent, UserComponentId, UserComponentSource,
    auxiliary::AuxiliaryData,
    imports::ResolvedImport,
    paths::{cannot_resolve_callable_path, invalid_config_type},
};
use crate::{
    compiler::{
        analyses::computations::ComputationDb,
        component::{ConfigType, DefaultStrategy},
        resolvers::{
            CallableResolutionError, GenericBindings, InputParameterResolutionError,
            OutputTypeResolutionError, SelfResolutionError, resolve_type,
        },
    },
    diagnostic::{DiagnosticSink, Registration},
    language::{
        Callable, FQPath, FQPathSegment, Generic, GenericArgument, GenericLifetimeParameter,
        InvocationStyle, PathType, ResolvedType,
    },
    rustdoc::{Crate, CrateCollection, GlobalItemId, RustdocKindExt},
};
use pavex_bp_schema::{
    CloningStrategy, CreatedAt, CreatedBy, Lifecycle, Lint, LintSetting, RawIdentifiers,
};
use pavexc_attr_parser::{AnnotationKind, AnnotationProperties};
use rustdoc_types::{Item, ItemEnum};

/// An id pointing at the coordinates of an annotated component.
pub type AnnotatedItemId = la_arena::Idx<GlobalItemId>;

/// Process all imported annotated components.
pub(super) fn register_imported_components(
    imported_modules: &[(ResolvedImport, usize)],
    aux: &mut AuxiliaryData,
    computation_db: &mut ComputationDb,
    registry: &AnnotationRegistry,
    krate_collection: &CrateCollection,
    diagnostics: &mut DiagnosticSink,
) {
    for (import, import_id) in imported_modules {
        let ResolvedImport {
            path: module_path,
            package_id,
        } = import;
        let scope_id = aux.imports[*import_id].1;
        let Some(krate) = krate_collection.get_crate_by_package_id(package_id) else {
            unreachable!(
                "The JSON documentation for packages that may contain annotated components \
                has already been generated at this point. If you're seeing this error, there's a bug in `pavexc`.\n\
                Please report this issue at https://github.com/LukeMathWalker/pavex/issues/new."
            )
        };
        // Let's check if the imported module path actually matches the path of a module in the
        // relevant crate.
        if !krate
            .import_index
            .modules
            .iter()
            .any(|(_, entry)| entry.defined_at.as_ref() == Some(module_path))
        {
            // No module matches. Perhaps it's another item kind?
            match krate
                .import_index
                .items
                .iter()
                .find(|(_, entry)| entry.defined_at.as_ref() == Some(module_path))
            {
                Some(_) => {
                    // We have a matching item. Let's report the kind confusion.
                    not_a_module(module_path, &aux.imports[*import_id].0, diagnostics);
                }
                None => {
                    // Nope, no match at all. Let's just report it as an unknown path.
                    unknown_module_path(
                        module_path,
                        &krate.crate_name(),
                        &aux.imports[*import_id].0,
                        diagnostics,
                    );
                }
            };
            continue;
        }

        let annotated_items = &registry[package_id];
        for (id, annotation) in annotated_items.iter() {
            // Not all components are auto-registered via imports.
            // In particular, those that are position-sensitive *must* be
            // registered individually by the user.
            let kind = annotation.properties.kind();
            if !(kind == AnnotationKind::Config || kind == AnnotationKind::Constructor) {
                continue;
            }

            // First check if the item is in scope for the import
            {
                let id = match &annotation.impl_ {
                    Some(impl_info) => impl_info.self_,
                    None => id,
                };
                let entry = &krate.import_index.items[&id];
                if !entry.paths().any(|path| path.starts_with(module_path)) {
                    continue;
                }
            }

            let item = krate.get_item_by_local_type_id(&id);
            let Ok(user_component_id) = intern_annotated(
                annotation.properties.clone(),
                &item,
                krate,
                &annotation
                    .created_at(krate, krate_collection.package_graph())
                    .expect("Failed to determine created at for an annotated item"),
                scope_id,
                aux,
                diagnostics,
                krate_collection,
            ) else {
                continue;
            };

            if !matches!(item.inner, ItemEnum::Function(_)) {
                continue;
            }

            let outcome = match annotation.impl_ {
                Some(ImplInfo { self_, impl_ }) => {
                    rustdoc_method2callable(self_, impl_, &item, krate, krate_collection)
                }
                None => rustdoc_free_fn2callable(&item, krate, krate_collection),
            };
            let callable = match outcome {
                Ok(callable) => callable,
                Err(e) => {
                    cannot_resolve_callable_path(
                        e,
                        user_component_id,
                        aux,
                        krate_collection.package_graph(),
                        diagnostics,
                    );
                    continue;
                }
            };
            computation_db.get_or_intern_with_id(callable, user_component_id.into());
        }
    }
}

/// Process the annotation and intern the associated component(s).
/// Returns the identifier of the newly interned component.
fn intern_annotated(
    annotation: AnnotationProperties,
    item: &rustdoc_types::Item,
    krate: &Crate,
    created_at: &CreatedAt,
    scope_id: ScopeId,
    aux: &mut AuxiliaryData,
    diagnostics: &mut DiagnosticSink,
    krate_collection: &CrateCollection,
) -> Result<UserComponentId, ()> {
    let registration = Registration::annotated_item(item, krate);
    let source: UserComponentSource = aux
        .annotation_interner
        .get_or_intern(GlobalItemId::new(item.id, krate.core.package_id.to_owned()))
        .into();

    match annotation {
        AnnotationProperties::Constructor {
            lifecycle,
            cloning_strategy,
            error_handler,
        } => {
            let constructor = UserComponent::Constructor { source };
            let constructor_id =
                aux.intern_component(constructor, scope_id, lifecycle, registration.clone());
            aux.id2cloning_strategy.insert(
                constructor_id,
                cloning_strategy.unwrap_or(CloningStrategy::NeverClone),
            );

            // Ignore unused constructors imported from crates defined outside the current workspace
            if !krate_collection
                .package_graph()
                .metadata(&krate.core.package_id)
                .unwrap()
                .in_workspace()
            {
                let mut lints = BTreeMap::new();
                lints.insert(Lint::Unused, LintSetting::Ignore);
                aux.id2lints.insert(constructor_id, lints);
            }

            if let Some(error_handler) = error_handler {
                let identifiers = RawIdentifiers {
                    created_at: created_at.clone(),
                    created_by: CreatedBy::macro_name("constructor"),
                    import_path: error_handler,
                };
                let identifiers_id = aux.identifiers_interner.get_or_intern(identifiers);
                let component = UserComponent::ErrorHandler {
                    source: identifiers_id.into(),
                    fallible_id: constructor_id,
                };
                aux.intern_component(component, scope_id, lifecycle, registration);
            }
            Ok(constructor_id)
        }
        AnnotationProperties::Config {
            key,
            cloning_strategy,
            default_if_missing,
            include_if_unused,
        } => {
            let config = UserComponent::ConfigType {
                key: key.clone(),
                source,
            };
            let config_id =
                aux.intern_component(config, scope_id, Lifecycle::Singleton, registration);
            aux.id2cloning_strategy.insert(
                config_id,
                cloning_strategy.unwrap_or(CloningStrategy::CloneIfNecessary),
            );
            let default_strategy = match default_if_missing {
                Some(true) => DefaultStrategy::DefaultIfMissing,
                Some(false) => DefaultStrategy::Required,
                None => Default::default(),
            };
            aux.config_id2default_strategy
                .insert(config_id, default_strategy);
            aux.config_id2include_if_unused
                .insert(config_id, include_if_unused.unwrap_or(false));

            let ty = match rustdoc_item_def2type(item, krate) {
                Ok(t) => t,
                Err(e) => {
                    const_generics_are_not_supported(e, item, diagnostics);
                    return Err(());
                }
            };
            match ConfigType::new(ty, key) {
                Ok(config) => {
                    aux.config_id2type.insert(config_id, config);
                }
                Err(e) => {
                    let path = FQPath {
                        segments: krate.import_index.items[&item.id]
                            .canonical_path()
                            .iter()
                            .cloned()
                            .map(FQPathSegment::new)
                            .collect(),
                        qualified_self: None,
                        package_id: krate.core.package_id.clone(),
                    };
                    invalid_config_type(e, &path, config_id, aux, diagnostics)
                }
            };

            Ok(config_id)
        }
        AnnotationProperties::PreProcessingMiddleware { .. }
        | AnnotationProperties::PostProcessingMiddleware { .. }
        | AnnotationProperties::ErrorObserver
        | AnnotationProperties::WrappingMiddleware { .. } => {
            unreachable!()
        }
    }
}

fn rustdoc_item_def2type(
    item: &Item,
    krate: &Crate,
) -> Result<ResolvedType, ConstGenericsAreNotSupported> {
    assert!(
        matches!(&item.inner, ItemEnum::Struct(_) | ItemEnum::Enum(_)),
        "Unexpected item type, `{}`. Expected a struct or enum.",
        item.inner.kind()
    );

    let path = krate.import_index.items[&item.id].canonical_path();

    let mut generic_arguments = vec![];
    let params_def = match &item.inner {
        ItemEnum::Struct(s) => &s.generics.params,
        ItemEnum::Enum(e) => &e.generics.params,
        _ => unreachable!(),
    };
    for arg in params_def {
        let arg = match &arg.kind {
            rustdoc_types::GenericParamDefKind::Lifetime { .. } => {
                let lifetime = arg.name.strip_prefix("'").unwrap_or(&arg.name);
                GenericArgument::Lifetime(GenericLifetimeParameter::Named(lifetime.to_owned()))
            }
            rustdoc_types::GenericParamDefKind::Type { .. } => {
                // TODO: Use the default if available.
                GenericArgument::TypeParameter(ResolvedType::Generic(Generic {
                    name: arg.name.clone(),
                }))
            }
            rustdoc_types::GenericParamDefKind::Const { .. } => todo!(),
        };
        generic_arguments.push(arg);
    }

    Ok(ResolvedType::ResolvedPath(PathType {
        package_id: krate.core.package_id.clone(),
        rustdoc_id: Some(item.id),
        base_type: path.into(),
        generic_arguments,
    }))
}

#[derive(Debug)]
struct ConstGenericsAreNotSupported {
    pub name: String,
}

/// Convert a free function from `rustdoc_types` into a `Callable`.
fn rustdoc_free_fn2callable(
    item: &Item,
    krate: &Crate,
    krate_collection: &CrateCollection,
) -> Result<Callable, CallableResolutionError> {
    let ItemEnum::Function(inner) = &item.inner else {
        unreachable!("Expected a function item");
    };
    let path = FQPath {
        segments: krate.import_index.items[&item.id]
            .canonical_path()
            .iter()
            .cloned()
            .map(FQPathSegment::new)
            .collect(),
        qualified_self: None,
        package_id: krate.core.package_id.clone(),
    };

    let mut inputs = Vec::new();
    for (parameter_index, (_, input_ty)) in inner.sig.inputs.iter().enumerate() {
        match resolve_type(
            input_ty,
            &krate.core.package_id,
            krate_collection,
            &Default::default(),
        ) {
            Ok(t) => {
                inputs.push(t);
            }
            Err(e) => {
                return Err(InputParameterResolutionError {
                    callable_path: path,
                    callable_item: item.clone(),
                    parameter_type: input_ty.clone(),
                    parameter_index,
                    source: Arc::new(e),
                }
                .into());
            }
        }
    }

    let output = match &inner.sig.output {
        Some(output_ty) => {
            match resolve_type(
                output_ty,
                &krate.core.package_id,
                krate_collection,
                &Default::default(),
            ) {
                Ok(t) => Some(t),
                Err(e) => {
                    return Err(OutputTypeResolutionError {
                        callable_path: path,
                        callable_item: item.clone(),
                        output_type: output_ty.clone(),
                        source: Arc::new(e),
                    }
                    .into());
                }
            }
        }
        None => None,
    };

    Ok(Callable {
        is_async: inner.header.is_async,
        // It's a free function, there's no `self`.
        takes_self_as_ref: false,
        output,
        path,
        inputs,
        invocation_style: InvocationStyle::FunctionCall,
        source_coordinates: Some(GlobalItemId {
            rustdoc_item_id: item.id,
            package_id: krate.core.package_id.clone(),
        }),
    })
}

fn rustdoc_method2callable(
    self_id: rustdoc_types::Id,
    impl_id: rustdoc_types::Id,
    method_item: &Item,
    krate: &Crate,
    krate_collection: &CrateCollection,
) -> Result<Callable, CallableResolutionError> {
    let method_path = {
        FQPath {
            segments: krate.import_index.items[&self_id]
                .canonical_path()
                .iter()
                .cloned()
                .map(FQPathSegment::new)
                .chain(std::iter::once(FQPathSegment::new(
                    method_item.name.clone().expect("Method without a name"),
                )))
                .collect(),
            qualified_self: None,
            package_id: krate.core.package_id.clone(),
        }
    };

    let ItemEnum::Function(inner) = &method_item.inner else {
        unreachable!("Expected a function item");
    };

    let impl_item = krate.get_item_by_local_type_id(&impl_id);
    let ItemEnum::Impl(impl_item) = &impl_item.inner else {
        unreachable!("The impl item id doesn't point to an impl item")
    };
    let self_ty = match resolve_type(
        &impl_item.for_,
        &krate.core.package_id,
        krate_collection,
        &Default::default(),
    ) {
        Ok(t) => t,
        Err(e) => {
            return Err(SelfResolutionError {
                path: method_path,
                source: Arc::new(e),
            }
            .into());
        }
    };

    let mut generic_bindings = GenericBindings::default();
    generic_bindings.types.insert("Self".into(), self_ty);

    let mut inputs = Vec::new();
    let mut takes_self_as_ref = false;
    for (parameter_index, (_, parameter_type)) in inner.sig.inputs.iter().enumerate() {
        if parameter_index == 0 {
            // The first parameter might be `&self` or `&mut self`.
            // This is important to know for carrying out further analysis doing the line,
            // e.g. undoing lifetime elision.
            if let rustdoc_types::Type::BorrowedRef { type_, .. } = parameter_type {
                if let rustdoc_types::Type::Generic(g) = type_.deref() {
                    if g == "Self" {
                        takes_self_as_ref = true;
                    }
                }
            }
        }

        match resolve_type(
            parameter_type,
            &krate.core.package_id,
            krate_collection,
            &generic_bindings,
        ) {
            Ok(t) => {
                inputs.push(t);
            }
            Err(e) => {
                return Err(InputParameterResolutionError {
                    callable_path: method_path,
                    callable_item: method_item.clone(),
                    parameter_type: parameter_type.clone(),
                    parameter_index,
                    source: Arc::new(e),
                }
                .into());
            }
        }
    }

    let output = match &inner.sig.output {
        Some(output_ty) => {
            match resolve_type(
                output_ty,
                &krate.core.package_id,
                krate_collection,
                &generic_bindings,
            ) {
                Ok(t) => Some(t),
                Err(e) => {
                    return Err(OutputTypeResolutionError {
                        callable_path: method_path,
                        callable_item: method_item.clone(),
                        output_type: output_ty.clone(),
                        source: Arc::new(e),
                    }
                    .into());
                }
            }
        }
        None => None,
    };

    Ok(Callable {
        is_async: inner.header.is_async,
        takes_self_as_ref,
        output,
        path: method_path,
        inputs,
        invocation_style: InvocationStyle::FunctionCall,
        source_coordinates: Some(GlobalItemId {
            rustdoc_item_id: method_item.id,
            package_id: krate.core.package_id.clone(),
        }),
    })
}
