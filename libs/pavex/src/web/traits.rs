use rustdoc_types::ItemEnum;

use crate::language::ResolvedType;
use crate::rustdoc::CrateCollection;

/// It returns `true` if `type_` implements the specified trait.
///
/// The trait path must be fully resolved: it should NOT point to a re-export
/// (e.g. `std::marker::Sync` won't work, you should use `core::marker::Sync`).
pub(crate) fn implements_trait(
    krate_collection: &CrateCollection,
    type_: &ResolvedType,
    expected_trait_path: &[&str],
) -> bool {
    let krate = krate_collection.get_crate_by_package_id(&type_.package_id);
    let type_id = krate.get_type_id_by_path(&type_.base_type).unwrap();
    let type_item = krate_collection.get_type_by_global_type_id(type_id);
    let impls = match &type_item.inner {
        ItemEnum::Struct(s) => &s.impls,
        ItemEnum::Enum(e) => &e.impls,
        _ => unreachable!(),
    };
    for impl_id in impls {
        let trait_id = match &krate.get_type_by_local_type_id(impl_id).inner {
            ItemEnum::Impl(impl_) => impl_.trait_.as_ref().map(|p| &p.id),
            _ => unreachable!(),
        };
        if let Some(trait_id) = trait_id {
            if let Ok((_, trait_path)) =
                krate_collection.get_canonical_path_by_local_type_id(&type_.package_id, &trait_id)
            {
                if trait_path == expected_trait_path {
                    return true;
                }
            }
        }
    }
    false
}
