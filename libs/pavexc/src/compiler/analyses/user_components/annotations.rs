use super::{
    UserComponent, auxiliary::AuxiliaryData, identifiers::ResolvedPaths, imports::ResolvedImport,
};
use crate::{
    diagnostic::{DiagnosticSink, Registration},
    rustdoc::{CrateCollection, GlobalItemId},
};
use pavex_bp_schema::{CloningStrategy, Import, RawIdentifiers};
use pavexc_attr_parser::AnnotatedComponent;

/// An id pointing at the coordinates of an annotated component.
pub type AnnotatedItemId = la_arena::Idx<GlobalItemId>;

/// Process all annotated components.
pub(super) fn register_imported_components(
    imported_modules: &[(ResolvedImport, usize)],
    aux: &mut AuxiliaryData,
    resolved_paths: &mut ResolvedPaths,
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
        let item_ids = krate
            .public_item_id2import_paths()
            .iter()
            .filter_map(|(id, paths)| {
                if paths.iter().any(|path| path.0.starts_with(module_path)) {
                    Some(id)
                } else {
                    None
                }
            });
        for item_id in item_ids {
            let item = krate.get_item_by_local_type_id(item_id);
            match pavexc_attr_parser::parse(&item.attrs) {
                Ok(Some(annotation)) => match annotation {
                    AnnotatedComponent::Constructor {
                        lifecycle,
                        cloning_strategy,
                        error_handler,
                    } => {
                        let constructor = UserComponent::Constructor {
                            source: aux
                                .annotation_interner
                                .get_or_intern(GlobalItemId::new(
                                    item_id.to_owned(),
                                    package_id.to_owned(),
                                ))
                                .into(),
                        };
                        let Some(span) = item.span.as_ref() else {
                            panic!(
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
                            aux.intern_component(component, scope_id, lifecycle, registration);
                        }
                    }
                },
                Ok(None) => {}
                Err(e) => {
                    // TODO: Handle the error
                }
            }
        }
    }

    // We resolve identifiers for all new components.
    for (id, _) in aux.component_interner.iter().skip(old_n_components) {
        resolved_paths.resolve(id, aux, krate_collection.package_graph(), diagnostics);
    }
}
