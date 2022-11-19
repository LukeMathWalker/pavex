use std::fmt::Formatter;

use rustdoc_types::ItemEnum;

use crate::language::ResolvedType;
use crate::rustdoc::CrateCollection;

/// It returns an error if `type_` does not implement the specified trait.
///
/// The trait path must be fully resolved: it should NOT point to a re-export
/// (e.g. `std::marker::Sync` won't work, you should use `core::marker::Sync`).
pub(crate) fn assert_trait_is_implemented(
    krate_collection: &CrateCollection,
    type_: &ResolvedType,
    expected_trait_path: &[&'static str],
) -> Result<(), MissingTraitImplementationError> {
    if !implements_trait(krate_collection, type_, expected_trait_path) {
        Err(MissingTraitImplementationError {
            type_: type_.to_owned(),
            trait_path: expected_trait_path.to_vec(),
        })
    } else {
        Ok(())
    }
}

/// It returns `true` if `type_` implements the specified trait.
///
/// The trait path must be fully resolved: it should NOT point to a re-export
/// (e.g. `std::marker::Sync` won't work, you should use `core::marker::Sync`).
pub(crate) fn implements_trait(
    krate_collection: &CrateCollection,
    type_: &ResolvedType,
    expected_trait_path: &[&'static str],
) -> bool {
    let krate = krate_collection.get_crate_by_package_id(&type_.package_id);
    let type_id = krate.get_type_id_by_path(&type_.base_type).unwrap();
    let type_item = krate_collection.get_type_by_global_type_id(type_id);
    let impls = match &type_item.inner {
        ItemEnum::Struct(s) => &s.impls,
        ItemEnum::Enum(e) => &e.impls,
        _ => unreachable!(),
    };
    // 2:3423:228
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

#[derive(Debug)]
pub(crate) struct MissingTraitImplementationError {
    pub type_: ResolvedType,
    pub trait_path: Vec<&'static str>,
}

impl std::error::Error for MissingTraitImplementationError {}

impl std::fmt::Display for MissingTraitImplementationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "`{:?}` does not implement the `{}` trait.",
            &self.type_,
            self.trait_path.join("::")
        )
    }
}
