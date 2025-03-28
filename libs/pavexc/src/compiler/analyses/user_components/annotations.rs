use std::{ops::Deref, sync::Arc};

use self::computations::ComputationDb;

use super::{
    ScopeId, UserComponent, UserComponentId, auxiliary::AuxiliaryData, identifiers::ResolvedPaths,
    imports::ResolvedImport, paths::cannot_resolve_callable_path,
};
use crate::{
    compiler::{
        analyses::computations,
        resolvers::{
            CallableResolutionError, GenericBindings, InputParameterResolutionError,
            OutputTypeResolutionError, SelfResolutionError, resolve_type,
        },
    },
    diagnostic::{DiagnosticSink, Registration, TargetSpan},
    language::{Callable, InvocationStyle, ResolvedPath, ResolvedPathSegment},
    rustdoc::{Crate, CrateCollection, GlobalItemId},
};
use pavex_bp_schema::{CloningStrategy, CreatedAt, Import, RawIdentifiers};
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
        let (Import { created_at, .. }, scope_id) = aux.imports[*import_id].clone();
        let Some(krate) = krate_collection.get_crate_by_package_id(package_id) else {
            unreachable!(
                "The JSON documentation for packages that may contain annotated components \
                has already been generated at this point. If you're seeing this error, there's a bug in `pavexc`.\n\
                Please report this issue at https://github.com/LukeMathWalker/pavex/issues/new."
            )
        };
        let mut queue: Vec<_> = krate
            .public_item_id2import_paths()
            .iter()
            .filter_map(|(id, paths)| {
                if paths.iter().any(|path| path.0.starts_with(module_path)) {
                    Some(QueueItem::Standalone(*id))
                } else {
                    None
                }
            })
            .collect();
        while let Some(queue_item) = queue.pop() {
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
                            // We don't have any annotation that goes directly on structs and enums (yet).
                            continue;
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
                            let user_component_id = intern_annotated(
                                annotation,
                                &item,
                                krate,
                                &created_at,
                                scope_id,
                                aux,
                            );
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
                    let user_component_id =
                        intern_annotated(annotation, &item, krate, &created_at, scope_id, aux);
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

/// Process the annotation and intern the associated component(s).
/// Returns the identifier of the newly interned component.
fn intern_annotated(
    annotation: AnnotatedComponent,
    item: &rustdoc_types::Item,
    krate: &Crate,
    created_at: &CreatedAt,
    scope_id: ScopeId,
    aux: &mut AuxiliaryData,
) -> UserComponentId {
    match annotation {
        AnnotatedComponent::Constructor {
            lifecycle,
            cloning_strategy,
            error_handler,
        } => {
            let constructor = UserComponent::Constructor {
                source: aux
                    .annotation_interner
                    .get_or_intern(GlobalItemId::new(item.id, krate.core.package_id.to_owned()))
                    .into(),
            };
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
            let constructor_id =
                aux.intern_component(constructor, scope_id, lifecycle, registration.clone());
            aux.id2cloning_strategy.insert(
                constructor_id,
                cloning_strategy.unwrap_or(CloningStrategy::NeverClone),
            );

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
            constructor_id
        }
    }
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
        segments: krate.public_item_id2import_paths()[&item.id]
            .first()
            .expect("No import paths for a publicly visible item.")
            .0
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
                    callable_path: path.into(),
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
                        callable_path: path.into(),
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
        let self_path = &krate.public_item_id2import_paths()[&self_id]
            .first()
            .expect("Publicly visible item without an import path")
            .0;
        ResolvedPath {
            segments: self_path
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

fn invalid_diagnostic_attribute(
    e: AttributeParserError,
    item: &Item,
    diagnostics: &mut DiagnosticSink,
) {
    let source = item
        .span
        .as_ref()
        .map(|s| {
            diagnostics.annotated(
                TargetSpan::Registration(&Registration::attribute(&s)),
                "The annotated item",
            )
        })
        .flatten();
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
