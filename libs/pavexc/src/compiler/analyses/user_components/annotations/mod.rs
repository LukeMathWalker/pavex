use std::{borrow::Cow, collections::BTreeMap, ops::Deref, sync::Arc};

mod diagnostic;

use diagnostic::{
    const_generics_are_not_supported, not_a_module, not_a_type_reexport, unknown_module_path,
    unresolved_external_reexport,
};

use super::{
    ErrorHandlerTarget, ScopeId, UserComponent, UserComponentId, UserComponentSource,
    auxiliary::AuxiliaryData,
    blueprint::validate_route_path,
    imports::{ImportKind, ResolvedImport},
    paths::{cannot_resolve_callable_path, invalid_config_type, invalid_prebuilt_type},
    router_key::RouterKey,
    scope_graph::ScopeGraphBuilder,
};
use crate::{
    compiler::{
        analyses::{computations::ComputationDb, prebuilt_types::PrebuiltTypeDb},
        component::{ConfigType, DefaultStrategy, PrebuiltType},
        resolvers::{
            CallableResolutionError, GenericBindings, InputParameterResolutionError,
            OutputTypeResolutionError, SelfResolutionError, resolve_type,
        },
    },
    diagnostic::{ComponentKind, DiagnosticSink, Registration},
    language::{
        Callable, FQGenericArgument, FQPath, FQPathSegment, FQQualifiedSelf, Generic,
        GenericArgument, GenericLifetimeParameter, InvocationStyle, PathType, ResolvedPathLifetime,
        ResolvedType,
    },
    rustdoc::{Crate, CrateCollection, GlobalItemId, ImplInfo, RustdocKindExt},
};
use pavex_bp_schema::{CloningStrategy, Lifecycle, Lint, LintSetting};
use pavexc_attr_parser::{AnnotationKind, AnnotationProperties};
use rustdoc_types::{GenericArgs, Item, ItemEnum};

/// An id pointing at the coordinates of an annotated component.
pub type AnnotatedItemId = la_arena::Idx<GlobalItemId>;

/// Process all imported annotated components.
pub(super) fn register_imported_components(
    imported_modules: &[(ResolvedImport, usize)],
    aux: &mut AuxiliaryData,
    scope_graph_builder: &mut ScopeGraphBuilder,
    computation_db: &mut ComputationDb,
    prebuilt_type_db: &mut PrebuiltTypeDb,
    krate_collection: &CrateCollection,
    diagnostics: &DiagnosticSink,
) {
    for (import, import_id) in imported_modules {
        let ResolvedImport {
            path: module_path,
            package_id,
            kind: import_kind,
            scope_id,
        } = import;
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
                    not_a_module(module_path, &aux.imports[*import_id], diagnostics);
                }
                None => {
                    // Nope, no match at all. Let's just report it as an unknown path.
                    unknown_module_path(
                        module_path,
                        &krate.crate_name(),
                        &aux.imports[*import_id],
                        diagnostics,
                    );
                }
            };
            continue;
        }

        let annotated_items = &krate_collection
            .get_crate_by_package_id(package_id)
            .unwrap()
            .annotated_items;
        for (id, annotation) in annotated_items.iter() {
            // Not all components are auto-registered via imports.
            // In particular, those that are position-sensitive *must* be
            // registered individually by the user.
            let kind = annotation.properties.kind();
            match kind {
                AnnotationKind::Prebuilt
                | AnnotationKind::Config
                | AnnotationKind::Constructor
                | AnnotationKind::ErrorHandler => {
                    if !matches!(import_kind, ImportKind::OrderIndependentComponents) {
                        continue;
                    }
                }
                AnnotationKind::Route => {
                    if !matches!(import_kind, ImportKind::Routes { .. }) {
                        continue;
                    }
                }
                AnnotationKind::WrappingMiddleware
                | AnnotationKind::PreProcessingMiddleware
                | AnnotationKind::Methods
                | AnnotationKind::PostProcessingMiddleware
                | AnnotationKind::Fallback
                | AnnotationKind::ErrorObserver => {
                    continue;
                }
            }

            // First check if the item is in scope for the import
            {
                let id = match &annotation.impl_ {
                    Some(impl_info) => impl_info.attached_to,
                    None => id,
                };
                let entry = krate.import_index.items.get(&id).unwrap_or_else(|| {
                    // It's a re-export.
                    let module_id = krate.import_index.re_export2parent_module[&id];
                    &krate.import_index.modules[&module_id]
                });
                if !entry.paths().any(|path| path.starts_with(module_path)) {
                    continue;
                }
            }

            // If an error handler is marked as "default = false", it should only be
            // used when explicitly set by the user as the desired error handler for
            // a given component via the `error_handler = ...` macro argument.
            if let AnnotationProperties::ErrorHandler {
                default: Some(false),
                ..
            } = annotation.properties
            {
                continue;
            }

            let item = krate.get_item_by_local_type_id(&id);
            let Ok(user_component_id) = intern_annotated(
                annotation.properties.clone(),
                &item,
                krate,
                import_kind,
                *scope_id,
                aux,
                scope_graph_builder,
                prebuilt_type_db,
                diagnostics,
                krate_collection,
            ) else {
                continue;
            };

            if !matches!(item.inner, ItemEnum::Function(_)) {
                continue;
            }

            let outcome = match annotation.impl_ {
                Some(ImplInfo { attached_to, impl_ }) => {
                    rustdoc_method2callable(attached_to, impl_, &item, krate, krate_collection)
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
    import_kind: &ImportKind,
    scope_id: ScopeId,
    aux: &mut AuxiliaryData,
    scope_graph_builder: &mut ScopeGraphBuilder,
    prebuilt_type_db: &mut PrebuiltTypeDb,
    diagnostics: &DiagnosticSink,
    krate_collection: &CrateCollection,
) -> Result<UserComponentId, ()> {
    let registration = Registration::annotated_item(item, krate);
    let source: UserComponentSource = aux
        .annotation_interner
        .get_or_intern(GlobalItemId::new(item.id, krate.core.package_id.to_owned()))
        .into();

    match annotation {
        AnnotationProperties::ErrorHandler {
            error_ref_input_index,
            default: _,
        } => {
            let error_handler = UserComponent::ErrorHandler {
                source,
                target: ErrorHandlerTarget::ErrorType {
                    error_ref_input_index,
                },
            };
            Ok(aux.intern_component(
                error_handler,
                scope_id,
                Lifecycle::Transient,
                registration.clone(),
            ))
        }
        AnnotationProperties::Constructor {
            lifecycle,
            cloning_strategy,
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

            Ok(constructor_id)
        }
        AnnotationProperties::Route { method, path } => {
            let ImportKind::Routes {
                path_prefix,
                domain_guard,
                observer_chain,
                middleware_chain,
            } = import_kind
            else {
                unreachable!()
            };
            let prefixed_path = if let Some(path_prefix) = path_prefix {
                format!("{path_prefix}{path}")
            } else {
                path.clone()
            };
            let router_key = RouterKey {
                path: prefixed_path,
                method_guard: method,
                domain_guard: domain_guard.clone(),
            };
            let request_handler = UserComponent::RequestHandler { source, router_key };

            let route_scope_id = scope_graph_builder.add_scope(scope_id, None);
            let request_handler_id = aux.intern_component(
                request_handler,
                route_scope_id,
                Lifecycle::RequestScoped,
                registration.clone(),
            );
            aux.handler_id2middleware_ids
                .insert(request_handler_id, middleware_chain.to_owned());
            aux.handler_id2error_observer_ids
                .insert(request_handler_id, observer_chain.to_owned());

            validate_route_path(aux, request_handler_id, &path, diagnostics);

            Ok(request_handler_id)
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

            let ty = annotated_item2type(item, krate, krate_collection, diagnostics)?;
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
        AnnotationProperties::Prebuilt { cloning_strategy } => {
            let prebuilt = UserComponent::PrebuiltType { source };
            let prebuilt_id =
                aux.intern_component(prebuilt, scope_id, Lifecycle::Singleton, registration);
            aux.id2cloning_strategy.insert(
                prebuilt_id,
                cloning_strategy.unwrap_or(CloningStrategy::CloneIfNecessary),
            );

            let ty = match rustdoc_item_def2type(item, krate) {
                Ok(t) => t,
                Err(e) => {
                    const_generics_are_not_supported(e, item, diagnostics);
                    return Err(());
                }
            };
            match PrebuiltType::new(ty) {
                Ok(prebuilt) => {
                    prebuilt_type_db.get_or_intern(prebuilt, prebuilt_id);
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
                    invalid_prebuilt_type(e, &path, prebuilt_id, aux, diagnostics)
                }
            };

            Ok(prebuilt_id)
        }
        AnnotationProperties::PreProcessingMiddleware { .. }
        | AnnotationProperties::PostProcessingMiddleware { .. }
        | AnnotationProperties::ErrorObserver
        | AnnotationProperties::Methods
        | AnnotationProperties::Fallback { .. }
        | AnnotationProperties::WrappingMiddleware { .. } => {
            unreachable!()
        }
    }
}

/// Given an annotation, retrieve and process the type it points at.
///
/// If the annotation is attached to a re-export, it is resolved
/// as part of the processing.
///
/// If procesing fails, it'll emit a diagnostic directly into the sink.
fn annotated_item2type(
    item: &Item,
    krate: &Crate,
    krate_collection: &CrateCollection,
    diagnostics: &DiagnosticSink,
) -> Result<ResolvedType, ()> {
    /// If the annotated item is a `use` statement, retrieve
    /// the definition of the re-exported item.
    fn annotated_item2def<'a>(
        item: &'a Item,
        krate: &'a Crate,
        krate_collection: &'a CrateCollection,
        diagnostics: &DiagnosticSink,
    ) -> Result<(Cow<'a, Item>, &'a Crate), ()> {
        let ItemEnum::Use(use_) = &item.inner else {
            // Not a re-export, the easy case.
            return Ok((Cow::Borrowed(item), krate));
        };
        let imported_id = use_.id.unwrap();
        let (source_krate, imported_item) =
            if let Some(item) = krate.maybe_get_item_by_local_type_id(&imported_id) {
                // Re-export of a local item.
                (krate, item)
            } else {
                // Re-export of an item defined in another crate.
                let imported_id = match krate
                    .external_re_exports
                    .get_target_item_id(krate, krate_collection, item.id)
                    .map_err(|_| {
                        // We failed to compute the crate docs.
                        // The collection will have emitted a diagnostic, so we can
                        // just return an `Err` here.
                    })? {
                    Some(imported_id) => imported_id,
                    None => {
                        // We failed to resolve the target item.
                        unresolved_external_reexport(item, ComponentKind::ConfigType, diagnostics);
                        return Err(());
                    }
                };
                let Ok(krate) =
                    krate_collection.get_or_compute_crate_by_package_id(&imported_id.package_id)
                else {
                    return Err(());
                };
                (
                    krate,
                    krate.get_item_by_local_type_id(&imported_id.rustdoc_item_id),
                )
            };
        match &imported_item.inner {
            ItemEnum::Enum(_) | ItemEnum::Struct(_) => {}
            other => {
                not_a_type_reexport(item, other.kind(), ComponentKind::ConfigType, diagnostics);
                return Err(());
            }
        }
        Ok((imported_item, source_krate))
    }

    let (item, krate) = annotated_item2def(item, krate, krate_collection, diagnostics)?;
    match rustdoc_item_def2type(&item, krate) {
        Ok(t) => Ok(t),
        Err(e) => {
            const_generics_are_not_supported(e, &item, diagnostics);
            Err(())
        }
    }
}

/// Convert an `enum` or a `struct` definition from the JSON documentation
/// for a crate into our own representation for types.
///
/// # Panics
///
/// Panics if the item isn't of kind `enum` or `struct`.
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

/// Convert a method item retrieved from `rustdoc`'s JSON output to Pavex's internal
/// representation for callables (i.e. methods and functions).
///
/// # Input constraints
///
/// - `method_item` belongs to `krate`.
/// - `impl_id` is local to `krate`.
/// - `attached_to` can either point to a trait or a type.
///   It'll always point to the `Self` type if we're working with an inherent method.
///
/// `attached_to`, in the trait case, may not be local to `krate`.
/// E.g. the user is implementing a trait defined in another crate
/// for one of their local types.
fn rustdoc_method2callable(
    attached_to: rustdoc_types::Id,
    impl_id: rustdoc_types::Id,
    method_item: &Item,
    krate: &Crate,
    krate_collection: &CrateCollection,
) -> Result<Callable, CallableResolutionError> {
    let impl_item = krate.get_item_by_local_type_id(&impl_id);
    let ItemEnum::Impl(impl_item) = &impl_item.inner else {
        unreachable!("The impl item id doesn't point to an impl item")
    };

    let mut generic_bindings = GenericBindings::default();

    let self_ty = match resolve_type(
        &impl_item.for_,
        &krate.core.package_id,
        krate_collection,
        &generic_bindings,
    ) {
        Ok(t) => t,
        Err(e) => {
            return Err(SelfResolutionError {
                // This path is not strictly correctly, since we may be dealing with a trait method,
                // but it's good enough for an error at this point in the flow.
                path: FQPath {
                    segments: krate.import_index.items[&attached_to]
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
                },
                source: Arc::new(e),
            }
            .into());
        }
    };

    generic_bindings
        .types
        .insert("Self".into(), self_ty.clone());

    let method_path = if let Some(trait_) = &impl_item.trait_ {
        let (trait_global_id, trait_path) = krate_collection
            .get_canonical_path_by_local_type_id(&krate.core.package_id, &trait_.id, None)
            // FIXME: handle the error
            .unwrap();
        let mut segments: Vec<_> = trait_path.iter().cloned().map(FQPathSegment::new).collect();
        let qualified_self = FQQualifiedSelf {
            position: segments.len(),
            type_: self_ty.into(),
        };
        let mut generic_args = Vec::new();
        if let Some(args) = &trait_.args {
            let GenericArgs::AngleBracketed { args, .. } = args.as_ref() else {
                // TODO: fixme.
                todo!();
            };
            for arg in args {
                let parsed_arg = match arg {
                    rustdoc_types::GenericArg::Lifetime(l) => {
                        let l = l.strip_prefix("'").unwrap_or(l.as_str());
                        if l == "static" {
                            FQGenericArgument::Lifetime(ResolvedPathLifetime::Static)
                        } else {
                            FQGenericArgument::Lifetime(ResolvedPathLifetime::Named(l.into()))
                        }
                    }
                    rustdoc_types::GenericArg::Type(t) => {
                        let Ok(t) = resolve_type(
                            t,
                            &krate.core.package_id,
                            krate_collection,
                            &generic_bindings,
                        ) else {
                            todo!()
                        };
                        FQGenericArgument::Type(t.into())
                    }
                    rustdoc_types::GenericArg::Const(_) => {
                        todo!()
                    }
                    rustdoc_types::GenericArg::Infer => {
                        // The placeholder `_` is not allowed within types on item signatures for implementations
                        unreachable!()
                    }
                };
                generic_args.push(parsed_arg);
            }
        }
        if let Some(last) = segments.last_mut() {
            last.generic_arguments = generic_args;
        }
        FQPath {
            segments: {
                segments.push(FQPathSegment::new(
                    method_item.name.clone().expect("Method without a name"),
                ));
                segments
            },
            qualified_self: Some(qualified_self),
            package_id: trait_global_id.package_id.clone(),
        }
    } else {
        FQPath {
            segments: krate.import_index.items[&attached_to]
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
