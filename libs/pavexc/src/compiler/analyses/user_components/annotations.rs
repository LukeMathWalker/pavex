use self::computations::ComputationDb;

use super::{
    UserComponent, auxiliary::AuxiliaryData, identifiers::ResolvedPaths, imports::ResolvedImport,
};
use crate::{
    compiler::{analyses::computations, resolvers::resolve_type},
    diagnostic::{DiagnosticSink, Registration},
    language::{
        Callable, InvocationStyle, ResolvedPath, ResolvedPathGenericArgument, ResolvedPathSegment,
    },
    rustdoc::{CrateCollection, GlobalItemId},
};
use pavex_bp_schema::{CloningStrategy, Import, RawIdentifiers};
use pavexc_attr_parser::AnnotatedComponent;
use rustdoc_types::{Enum, ItemEnum, Struct};

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
                        ItemEnum::Function(inner) => {
                            let annotation = match pavexc_attr_parser::parse(&item.attrs) {
                                Ok(Some(annotation)) => annotation,
                                Ok(None) => {
                                    continue;
                                }
                                Err(e) => {
                                    todo!("Failed to parse attributes: {}", e);
                                }
                            };
                            let global_item_id =
                                GlobalItemId::new(item_id.to_owned(), package_id.to_owned());
                            let user_component_id = match annotation {
                                AnnotatedComponent::Constructor {
                                    lifecycle,
                                    cloning_strategy,
                                    error_handler,
                                } => {
                                    let constructor = UserComponent::Constructor {
                                        source: aux
                                            .annotation_interner
                                            .get_or_intern(global_item_id.clone())
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
                                    let constructor_id = aux.intern_component(
                                        constructor,
                                        scope_id,
                                        lifecycle,
                                        registration.clone(),
                                    );
                                    aux.id2cloning_strategy.insert(
                                        constructor_id,
                                        cloning_strategy.unwrap_or(CloningStrategy::NeverClone),
                                    );

                                    if let Some(error_handler) = error_handler {
                                        let identifiers = RawIdentifiers {
                                            created_at: created_at.clone(),
                                            import_path: error_handler,
                                        };
                                        let identifiers_id =
                                            aux.identifiers_interner.get_or_intern(identifiers);
                                        let component = UserComponent::ErrorHandler {
                                            source: identifiers_id.into(),
                                            fallible_id: constructor_id,
                                        };
                                        aux.intern_component(
                                            component,
                                            scope_id,
                                            lifecycle,
                                            registration,
                                        );
                                    }
                                    constructor_id
                                }
                            };

                            let segments: Vec<_> = krate.public_item_id2import_paths()[&item_id]
                                .first()
                                .expect("No import paths for a publicly visible item.")
                                .0
                                .iter()
                                .map(|s| ResolvedPathSegment {
                                    ident: s.into(),
                                    generic_arguments: Vec::new(),
                                })
                                .collect();
                            let path = ResolvedPath {
                                segments,
                                qualified_self: None,
                                package_id: global_item_id.package_id.clone(),
                            };

                            let mut inputs = Vec::new();
                            for (_, input_ty) in &inner.sig.inputs {
                                match resolve_type(
                                    input_ty,
                                    &global_item_id.package_id,
                                    krate_collection,
                                    &Default::default(),
                                ) {
                                    Ok(t) => {
                                        inputs.push(t);
                                    }
                                    Err(e) => todo!("Failed to resolve input type: {}", e),
                                }
                            }

                            let output = match &inner.sig.output {
                                Some(output_ty) => {
                                    match resolve_type(
                                        output_ty,
                                        &global_item_id.package_id,
                                        krate_collection,
                                        &Default::default(),
                                    ) {
                                        Ok(t) => Some(t),
                                        Err(e) => todo!("Failed to resolve output type: {}", e),
                                    }
                                }
                                None => None,
                            };

                            let callable = Callable {
                                is_async: inner.header.is_async,
                                // It's a free function, there's no `self`.
                                takes_self_as_ref: false,
                                output,
                                path,
                                inputs,
                                invocation_style: InvocationStyle::FunctionCall,
                                source_coordinates: Some(global_item_id),
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
                // We'll deal with impls later.
                QueueItem::Impl { .. } => continue,
                QueueItem::ImplItem { .. } => continue,
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
