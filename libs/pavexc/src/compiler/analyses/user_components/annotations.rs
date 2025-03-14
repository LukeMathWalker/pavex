use super::{auxiliary::AuxiliaryData, imports::ResolvedImport};
use crate::{diagnostic::DiagnosticSink, rustdoc::CrateCollection};
use guppy::PackageId;

/// The identifier of the interned annotation metadata.
pub type AnnotationId = la_arena::Idx<AnnotationIdentifiers>;

/// Information required to retrieve the annotated item from JSON documentation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnnotationIdentifiers {
    /// The package ID of the crate that defined the annotated item.
    package_id: guppy::PackageId,
    /// The ID of the item within the crate associated with
    /// [`Self::package_id`].
    item_id: rustdoc_types::Id,
}

/// Process all annotated components.
pub(super) fn process_imported_modules(
    imported_modules: &[(ResolvedImport, usize)],
    aux: &mut AuxiliaryData,
    krate_collection: &CrateCollection,
    diagnostics: &mut DiagnosticSink,
) {
    for (import, import_id) in imported_modules {
        let ResolvedImport {
            path: module_path,
            package_id,
        } = import;
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
            for attr in item.attrs {
                pavexc_attr_parser::parse_attribute(&attr);
            }
        }
    }
}
