use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Deref,
    sync::Arc,
};

use self::computations::ComputationDb;

use super::{
    ScopeId, UserComponent, UserComponentId, UserComponentSource,
    auxiliary::AuxiliaryData,
    identifiers::ResolvedPaths,
    imports::ResolvedImport,
    paths::{cannot_resolve_callable_path, invalid_config_type},
};
use crate::{
    compiler::{
        analyses::computations,
        component::{ConfigType, DefaultStrategy},
        resolvers::{
            CallableResolutionError, GenericBindings, InputParameterResolutionError,
            OutputTypeResolutionError, SelfResolutionError, resolve_type,
        },
    },
    diagnostic::{
        self, ComponentKind, DiagnosticSink, OptionalLabeledSpanExt, OptionalSourceSpanExt,
        Registration, TargetSpan,
    },
    language::{
        Callable, Generic, GenericArgument, GenericLifetimeParameter, InvocationStyle, PathType,
        ResolvedPath, ResolvedPathSegment, ResolvedType,
    },
    rustdoc::{Crate, CrateCollection, GlobalItemId, RustdocKindExt},
};
use itertools::Itertools;
use pavex_bp_schema::{
    CloningStrategy, CreatedAt, Import, Lifecycle, Lint, LintSetting, RawIdentifiers,
};
use pavex_cli_diagnostic::CompilerDiagnostic;
use pavexc_attr_parser::{AnnotatedComponent, errors::AttributeParserError};
use rustdoc_types::{Enum, Item, ItemEnum, Struct};

/// An id pointing at the coordinates of an annotated component.
pub type AnnotatedItemId = la_arena::Idx<GlobalItemId>;

/// Process all annotated components.
pub(super) fn register_imported_components(
    imported_modules: &[(ResolvedImport, usize)],
    aux: &mut AuxiliaryData,
    resolved_paths: &mut ResolvedPaths,
    computation_db: &mut ComputationDb,
    krate_collection: &CrateCollection,
    diagnostics: &mut DiagnosticSink,
) {
    let old_n_components = aux.component_interner.len();
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
        // We use a BTreeSet to guarantee a deterministic processing order.
        let mut queue: BTreeSet<_> = krate
            .import_index
            .items
            .iter()
            .filter_map(|(id, entry)| {
                if entry.is_public() && entry.paths().any(|path| path.starts_with(module_path)) {
                    Some(QueueItem::Standalone(*id))
                } else {
                    None
                }
            })
            .collect();
        while let Some(queue_item) = queue.pop_last() {
            match queue_item {
                QueueItem::Standalone(item_id) => {
                    let item = krate.get_item_by_local_type_id(&item_id);
                    match &item.inner {
                        ItemEnum::Struct(Struct { impls, .. })
                        | ItemEnum::Enum(Enum { impls, .. }) => {
                            queue.extend(impls.iter().map(|impl_id| QueueItem::Impl {
                                self_: item_id,
                                id: *impl_id,
                            }));

                            let annotation = match pavexc_attr_parser::parse(&item.attrs) {
                                Ok(Some(annotation)) => annotation,
                                Ok(None) => {
                                    continue;
                                }
                                Err(e) => {
                                    invalid_diagnostic_attribute(e, item.as_ref(), diagnostics);
                                    continue;
                                }
                            };
                            match annotation {
                                AnnotatedComponent::Constructor { .. } => {
                                    unsupported_item_kind(
                                        annotation.attribute(),
                                        &item,
                                        diagnostics,
                                    );
                                    continue;
                                }
                                AnnotatedComponent::Config { .. } => {}
                            }

                            let _ = intern_annotated(
                                annotation,
                                &item,
                                krate,
                                &queue_item.created_at(krate).unwrap(),
                                scope_id,
                                aux,
                                diagnostics,
                                krate_collection,
                            );
                        }
                        ItemEnum::Function(_) => {
                            let annotation = match pavexc_attr_parser::parse(&item.attrs) {
                                Ok(Some(annotation)) => annotation,
                                Ok(None) => {
                                    continue;
                                }
                                Err(e) => {
                                    invalid_diagnostic_attribute(e, item.as_ref(), diagnostics);
                                    continue;
                                }
                            };
                            match annotation {
                                AnnotatedComponent::Constructor { .. } => {}
                                AnnotatedComponent::Config { .. } => {
                                    unsupported_item_kind(
                                        annotation.attribute(),
                                        &item,
                                        diagnostics,
                                    );
                                    continue;
                                }
                            }

                            let Ok(user_component_id) = intern_annotated(
                                annotation,
                                &item,
                                krate,
                                &queue_item.created_at(krate).unwrap(),
                                scope_id,
                                aux,
                                diagnostics,
                                krate_collection,
                            ) else {
                                continue;
                            };
                            let callable =
                                match rustdoc_free_fn2callable(&item, krate, krate_collection) {
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
                            computation_db
                                .get_or_intern_with_id(callable, user_component_id.into());
                        }
                        ItemEnum::Trait(_) => {
                            // Skip trait items for now.
                            continue;
                        }
                        _ => {
                            // Nothing else we care about.
                            continue;
                        }
                    };
                }
                QueueItem::Impl { self_, id } => {
                    let impl_item = krate.get_item_by_local_type_id(&id);
                    let ItemEnum::Impl(impl_) = &impl_item.inner else {
                        continue;
                    };
                    queue.extend(impl_.items.iter().map(|&item_id| QueueItem::ImplItem {
                        self_,
                        id: item_id,
                        impl_: id,
                    }));
                }
                QueueItem::ImplItem { self_, impl_, id } => {
                    let item = krate.get_item_by_local_type_id(&id);
                    let ItemEnum::Function(_) = &item.inner else {
                        continue;
                    };
                    let annotation = match pavexc_attr_parser::parse(&item.attrs) {
                        Ok(Some(annotation)) => annotation,
                        Ok(None) => {
                            continue;
                        }
                        Err(e) => {
                            invalid_diagnostic_attribute(e, item.as_ref(), diagnostics);
                            continue;
                        }
                    };
                    match annotation {
                        AnnotatedComponent::Constructor { .. } => {}
                        AnnotatedComponent::Config { .. } => {
                            unsupported_item_kind(annotation.attribute(), &item, diagnostics);
                            continue;
                        }
                    }

                    let Ok(user_component_id) = intern_annotated(
                        annotation,
                        &item,
                        krate,
                        &queue_item.created_at(krate).unwrap(),
                        scope_id,
                        aux,
                        diagnostics,
                        krate_collection,
                    ) else {
                        continue;
                    };
                    let callable =
                        match rustdoc_method2callable(self_, impl_, &item, krate, krate_collection)
                        {
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
    }

    // We resolve identifiers for all new components.
    for (id, _) in aux.component_interner.iter().skip(old_n_components) {
        resolved_paths.resolve(id, aux, krate_collection.package_graph(), diagnostics);
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum QueueItem {
    /// The `id` of an enum, struct, trait or function.
    Standalone(rustdoc_types::Id),
    Impl {
        /// The `id` of the `Self` type for this `impl` block.
        self_: rustdoc_types::Id,
        /// The `id` of the `impl` block item.
        id: rustdoc_types::Id,
    },
    ImplItem {
        /// The `id` of the `Self` type for this `impl` block.
        self_: rustdoc_types::Id,
        /// The `id` of the `impl` block that this item belongs to.
        impl_: rustdoc_types::Id,
        /// The `id` of the `impl` block item.
        id: rustdoc_types::Id,
    },
}

impl QueueItem {
    /// Returns the annotation location metadata.
    fn created_at(&self, krate: &Crate) -> Option<CreatedAt> {
        let id = match &self {
            QueueItem::Standalone(id) => *id,
            QueueItem::Impl { .. } => {
                return None;
            }
            QueueItem::ImplItem { self_, .. } => {
                // FIXME: The `impl` where this method is defined may not be within the same module
                // where `Self` is defined.
                // See https://rust-lang.zulipchat.com/#narrow/channel/266220-t-rustdoc/topic/Module.20items.20don't.20link.20to.20impls.20.5Brustdoc-json.5D
                // for a discussion on this issue.
                *self_
            }
        };
        let item = krate.get_item_by_local_type_id(&id);
        match &item.inner {
            ItemEnum::Struct(..) | ItemEnum::Enum(..) | ItemEnum::Function(..) => {
                let module_path = {
                    let fn_path = krate.import_index.items[&item.id]
                        .defined_at
                        .as_ref()
                        .expect("No `defined_at` in the import index for a struct/enum/function/method item.");
                    fn_path.iter().take(fn_path.len() - 1).join("::")
                };
                Some(CreatedAt {
                    crate_name: krate.crate_name(),
                    module_path,
                })
            }
            _ => None,
        }
    }
}

/// A lot of unnecessary jumping through hoops to implement `Ord`/`PartialOrd`
/// since `rustdoc_types::Id` doesn't implement `Ord`/`PartialOrd`.
mod sortable_queue {
    use super::QueueItem;
    impl QueueItem {
        fn as_sortable(&self) -> (SortableId, Option<SortableId>, Option<SortableId>) {
            match self {
                QueueItem::Standalone(id) => ((*id).into(), None, None),
                QueueItem::Impl { self_, id } => ((*self_).into(), Some((*id).into()), None),
                QueueItem::ImplItem { self_, impl_, id } => {
                    ((*self_).into(), Some((*impl_).into()), Some((*id).into()))
                }
            }
        }
    }

    impl PartialOrd for QueueItem {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for QueueItem {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            let sortable_self = self.as_sortable();
            let sortable_other = other.as_sortable();
            sortable_self.cmp(&sortable_other)
        }
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub struct SortableId(rustdoc_types::Id);

    impl From<rustdoc_types::Id> for SortableId {
        fn from(value: rustdoc_types::Id) -> Self {
            Self(value)
        }
    }

    impl PartialOrd for SortableId {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for SortableId {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.0.0.cmp(&other.0.0)
        }
    }
}

/// Process the annotation and intern the associated component(s).
/// Returns the identifier of the newly interned component.
fn intern_annotated(
    annotation: AnnotatedComponent,
    item: &rustdoc_types::Item,
    krate: &Crate,
    created_at: &CreatedAt,
    scope_id: ScopeId,
    aux: &mut AuxiliaryData,
    diagnostics: &mut DiagnosticSink,
    krate_collection: &CrateCollection,
) -> Result<UserComponentId, ()> {
    let Some(span) = item.span.as_ref() else {
        // TODO: We have empirically verified that this shouldn't happen for components annotated with our own macros,
        //   but it may happen for components that are generated from other macros or tools.
        //   In the future, we should handle this case more gracefully.
        unreachable!(
            "There is no span attached to the item for `{}` in the JSON documentation for `{}`",
            item.name.as_deref().unwrap_or(""),
            krate.crate_name()
        );
    };
    let registration = Registration::attribute(span);
    let source: UserComponentSource = aux
        .annotation_interner
        .get_or_intern(GlobalItemId::new(item.id, krate.core.package_id.to_owned()))
        .into();

    match annotation {
        AnnotatedComponent::Constructor {
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
        AnnotatedComponent::Config {
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
            match ConfigType::new(ty, key.into()) {
                Ok(config) => {
                    aux.config_id2type.insert(config_id, config);
                }
                Err(e) => {
                    let path = ResolvedPath {
                        segments: krate.import_index.items[&item.id]
                            .canonical_path()
                            .iter()
                            .cloned()
                            .map(ResolvedPathSegment::new)
                            .collect(),
                        qualified_self: None,
                        package_id: krate.core.package_id.clone(),
                    };
                    invalid_config_type(e, &path, config_id, aux, diagnostics)
                }
            };

            Ok(config_id)
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
    name: String,
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
    let path = ResolvedPath {
        segments: krate.import_index.items[&item.id]
            .canonical_path()
            .iter()
            .cloned()
            .map(ResolvedPathSegment::new)
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
        ResolvedPath {
            segments: krate.import_index.items[&self_id]
                .canonical_path()
                .iter()
                .cloned()
                .map(ResolvedPathSegment::new)
                .chain(std::iter::once(ResolvedPathSegment::new(
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

fn const_generics_are_not_supported(
    e: ConstGenericsAreNotSupported,
    item: &Item,
    diagnostics: &mut DiagnosticSink,
) {
    let source = item.span.as_ref().and_then(|s| {
        diagnostics.annotated(
            TargetSpan::Registration(&Registration::attribute(s), ComponentKind::Constructor),
            "The annotated item",
        )
    });
    let const_name = e.name;
    let err_msg = match &item.name {
        Some(name) => {
            format!(
                "Pavex does not support const generics.\n`{name}` uses at least one const generic parameter, named `{const_name}`.",
            )
        }
        None => format!(
            "Pavex does not support const generics.\nOne of your types uses at least one const generic parameter, named `{const_name}`."
        ),
    };
    let diagnostic = CompilerDiagnostic::builder(anyhow::anyhow!(err_msg))
        .optional_source(source)
        .help("Remove the const generic parameter from your type definition, or use a different type.".into())
        .build();
    diagnostics.push(diagnostic);
}

fn invalid_diagnostic_attribute(
    e: AttributeParserError,
    item: &Item,
    diagnostics: &mut DiagnosticSink,
) {
    let source = item.span.as_ref().and_then(|s| {
        diagnostics.annotated(
            TargetSpan::Registration(&Registration::attribute(s), ComponentKind::Constructor),
            "The annotated item",
        )
    });
    let err_msg = match &item.name {
        Some(name) => {
            format!("`{name}` is annotated with a malformed `diagnostic::pavex::*` attribute.",)
        }
        None => "One of your items is annotated with a malformed `diagnostic::pavex::*` attribute."
            .into(),
    };
    let diagnostic = CompilerDiagnostic::builder(anyhow::anyhow!(e.to_string()).context(err_msg))
        .optional_source(source)
        .help("Have you manually added the `diagnostic::pavex::*` attribute on the item? \
            The syntax for `diagnostic::pavex::*` attributes is an implementation detail of Pavex's own macros,
            which are guaranteed to output well-formed annotations.".into())
        .build();
    diagnostics.push(diagnostic);
}

fn unknown_module_path(
    module_path: &[String],
    krate_name: &str,
    import: &Import,
    diagnostics: &mut DiagnosticSink,
) {
    let source = diagnostics.source(&import.registered_at).map(|s| {
        diagnostic::imported_sources_span(s.source(), &import.registered_at)
            .labeled("The import was registered here".into())
            .attach(s)
    });
    let module_path = module_path.join("::");
    let e = anyhow::anyhow!(
        "You tried to import items from `{module_path}`, but there is no module with that path in `{krate_name}`."
    );
    let diagnostic = CompilerDiagnostic::builder(e)
        .optional_source(source)
        .build();
    diagnostics.push(diagnostic);
}

fn not_a_module(path: &[String], import: &Import, diagnostics: &mut DiagnosticSink) {
    let source = diagnostics.source(&import.registered_at).map(|s| {
        diagnostic::imported_sources_span(s.source(), &import.registered_at)
            .labeled("The import was registered here".into())
            .attach(s)
    });
    let path = path.join("::");
    let e =
        anyhow::anyhow!("You tried to import items from `{path}`, but `{path}` is not a module.");
    let diagnostic = CompilerDiagnostic::builder(e)
        .optional_source(source)
        .help(
            "Pass to `from!` the path to a module that contains the item you want to import, \
            rather than the path to the actual item."
                .into(),
        )
        .build();
    diagnostics.push(diagnostic);
}

fn unsupported_item_kind(attribute: &str, item: &Item, diagnostics: &mut DiagnosticSink) {
    let source = item.span.as_ref().and_then(|s| {
        diagnostics.annotated(
            TargetSpan::Registration(&Registration::attribute(s), ComponentKind::Constructor),
            "The annotated item",
        )
    });
    let err = match &item.name {
        Some(name) => {
            format!(
                "`{name}` is annotated with `{attribute}`, but `{attribute}` is not supported on {}.",
                item.inner.kind()
            )
        }
        None => format!("`{attribute}` is not supported on {}.", item.inner.kind()),
    };
    let diagnostic = CompilerDiagnostic::builder(anyhow::anyhow!(err))
        .optional_source(source)
        .help(format!("Have you manually added `{attribute}`? \
            The syntax for `diagnostic::pavex::*` attributes is an implementation detail of Pavex's own macros,
            which are guaranteed to output well-formed annotations."))
        .build();
    diagnostics.push(diagnostic);
}
