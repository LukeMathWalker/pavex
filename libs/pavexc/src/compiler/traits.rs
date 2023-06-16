use std::fmt::Formatter;

use ahash::{HashMap, HashMapExt};
use guppy::PackageId;
use rustdoc_types::{GenericParamDefKind, ItemEnum, Type};

use crate::compiler::resolvers::resolve_type;
use crate::language::{PathType, ResolvedType};
use crate::rustdoc::{Crate, CrateCollection};

/// It returns an error if `type_` doesn't implement the specified trait.
///
/// The trait path must be fully resolved: it should NOT point to a re-export
/// (e.g. `std::marker::Sync` won't work, you should use `core::marker::Sync`).
pub(crate) fn assert_trait_is_implemented(
    krate_collection: &CrateCollection,
    type_: &ResolvedType,
    expected_trait: &PathType,
) -> Result<(), MissingTraitImplementationError> {
    match implements_trait(krate_collection, type_, expected_trait) {
        Ok(implements) => {
            if implements {
                Ok(())
            } else {
                Err(MissingTraitImplementationError {
                    type_: type_.to_owned(),
                    trait_: expected_trait.to_owned().into(),
                })
            }
        }
        Err(e) => {
            tracing::trace!(
                "Failing to determine if `{:?}` implements `{:?}`. Assuming it does not—{:?}",
                type_,
                expected_trait,
                e
            );
            Err(MissingTraitImplementationError {
                type_: type_.to_owned(),
                trait_: expected_trait.to_owned().into(),
            })
        }
    }
}

fn get_crate_by_package_id<'a>(
    krate_collection: &'a CrateCollection,
    package_id: &'a PackageId,
) -> Result<&'a Crate, anyhow::Error> {
    krate_collection
        .get_crate_by_package_id(package_id)
        .ok_or_else(|| anyhow::anyhow!("Unknown package id: {}", package_id))
}

const COPY_TRAIT_PATH: [&str; 3] = ["core", "marker", "Copy"];
const SEND_TRAIT_PATH: [&str; 3] = ["core", "marker", "Send"];
const SYNC_TRAIT_PATH: [&str; 3] = ["core", "marker", "Sync"];
const UNPIN_TRAIT_PATH: [&str; 3] = ["core", "marker", "Unpin"];
const CLONE_TRAIT_PATH: [&str; 3] = ["core", "clone", "Clone"];

/// It returns `true` if `type_` implements the specified trait.
///
/// The trait path must be fully resolved: it should NOT point to a re-export
/// (e.g. `std::marker::Sync` won't work, you should use `core::marker::Sync`).
pub(crate) fn implements_trait(
    krate_collection: &CrateCollection,
    type_: &ResolvedType,
    expected_trait: &PathType,
) -> Result<bool, anyhow::Error> {
    let trait_definition_crate =
        get_crate_by_package_id(krate_collection, &expected_trait.package_id)?;
    let trait_item_id = trait_definition_crate
        .get_type_id_by_path(&expected_trait.base_type, krate_collection)??;
    let trait_item = krate_collection.get_type_by_global_type_id(&trait_item_id);
    let ItemEnum::Trait(trait_item) = &trait_item.inner else { unreachable!() };

    // Due to Rust's orphan rule, a trait implementation for a type can live in two places:
    // - In the crate where the type was defined;
    // - In the crate where the trait was defined.
    // We start by checking if there is a trait implementation for this type in the crate where the
    // type was defined.
    match type_ {
        ResolvedType::ResolvedPath(our_path_type) => {
            let type_definition_crate =
                get_crate_by_package_id(krate_collection, &our_path_type.package_id)?;
            let type_id = type_definition_crate
                .get_type_id_by_path(&our_path_type.base_type, krate_collection)??;
            let type_item = krate_collection.get_type_by_global_type_id(&type_id);
            // We want to see through type aliases here.
            if let ItemEnum::Typedef(typedef) = &type_item.inner {
                let mut generic_bindings = HashMap::new();
                for generic in &typedef.generics.params {
                    // We also try to handle generic parameters, as long as they have a default value.
                    match &generic.kind {
                        GenericParamDefKind::Type {
                            default: Some(default),
                            ..
                        } => {
                            let default = resolve_type(
                                default,
                                &our_path_type.package_id,
                                krate_collection,
                                &generic_bindings,
                            )?;
                            generic_bindings.insert(generic.name.to_string(), default);
                        }
                        GenericParamDefKind::Type { default: None, .. }
                        | GenericParamDefKind::Const { .. }
                        | GenericParamDefKind::Lifetime { .. } => {
                            todo!("Generic parameters other than type parameters with a default value are not supported yet. I can't handle:\n {:?}", generic)
                        }
                    }
                }
                let type_ = resolve_type(
                    &typedef.type_,
                    &our_path_type.package_id,
                    krate_collection,
                    &generic_bindings,
                )?;
                if implements_trait(krate_collection, &type_, expected_trait)? {
                    return Ok(true);
                }
            }
            let impls = match &type_item.inner {
                ItemEnum::Struct(s) => &s.impls,
                ItemEnum::Enum(e) => &e.impls,
                n => {
                    dbg!(n);
                    unreachable!()
                }
            };
            for impl_id in impls {
                let item = type_definition_crate.get_type_by_local_type_id(impl_id);
                let (trait_id, implementer_type) = match &item.inner {
                    ItemEnum::Impl(impl_) => {
                        if impl_.negative {
                            continue;
                        }
                        (impl_.trait_.as_ref().map(|p| &p.id), &impl_.for_)
                    }
                    _ => unreachable!(),
                };
                if let Some(trait_id) = trait_id {
                    if let Ok((_, trait_path)) = krate_collection
                        .get_canonical_path_by_local_type_id(&our_path_type.package_id, trait_id)
                    {
                        if trait_path == expected_trait.base_type
                            // The "impls" for a rustdoc item include implementations for
                            // references to the type!
                            // Therefore we must verify that the implementer type is indeed the
                            // "owned" version of our type.
                            && is_equivalent(implementer_type, type_, krate_collection, &our_path_type.package_id)
                        {
                            return Ok(true);
                        }
                    }
                }
            }
        }
        ResolvedType::Tuple(t) => {
            // Tuple trait implementations in std are somewhat magical
            // (see https://doc.rust-lang.org/std/primitive.tuple.html#trait-implementations-1).
            // We handle the ones we know we care about (marker traits and Clone).
            if (expected_trait.base_type == SEND_TRAIT_PATH
                || expected_trait.base_type == SYNC_TRAIT_PATH
                || expected_trait.base_type == COPY_TRAIT_PATH
                || expected_trait.base_type == UNPIN_TRAIT_PATH
                || expected_trait.base_type == CLONE_TRAIT_PATH)
                && t.elements
                    .iter()
                    .all(|t| match implements_trait(krate_collection, t, expected_trait) {
                        Ok(t) => t,
                        Err(e) => {
                            tracing::trace!("Failing to determine if `{:?}` implements `{:?}`. Assuming it does not—{:?}", t, expected_trait, e);
                            false
                        }
                    })
            {
                return Ok(true);
            }
        }
        ResolvedType::Reference(r) => {
            // `& &T` is `Send` if `&T` is `Send`, therefore `&T` is `Sync` if `T` if `Sync`.
            if (expected_trait.base_type == SYNC_TRAIT_PATH
                || expected_trait.base_type == SEND_TRAIT_PATH)
                && implements_trait(krate_collection, &r.inner, expected_trait)?
            {
                return Ok(true);
            }
            // `&T` is always `Copy`, but `&mut T` is never `Copy`.
            // See https://doc.rust-lang.org/std/marker/trait.Copy.html#impl-Copy-for-%26T and
            // https://doc.rust-lang.org/std/marker/trait.Copy.html#when-cant-my-type-be-copy
            // `Copy` implies `Clone`.
            if !r.is_mutable && (expected_trait.base_type == COPY_TRAIT_PATH)
                || (expected_trait.base_type == CLONE_TRAIT_PATH)
            {
                return Ok(true);
            }
            // TODO: Unpin and other traits
        }
        ResolvedType::ScalarPrimitive(_) => {

            if expected_trait.base_type == SEND_TRAIT_PATH
                || expected_trait.base_type == SYNC_TRAIT_PATH
                || expected_trait.base_type == COPY_TRAIT_PATH
                || expected_trait.base_type == UNPIN_TRAIT_PATH
                || expected_trait.base_type == CLONE_TRAIT_PATH
            {
                return Ok(true);
            }
            // TODO: handle other traits
        }
        ResolvedType::Slice(s) => {
            if (expected_trait.base_type == SEND_TRAIT_PATH
                || expected_trait.base_type == SYNC_TRAIT_PATH)
                && implements_trait(krate_collection, &s.element_type, expected_trait)?
            {
                return Ok(true);
            }
            // TODO: handle Unpin + other traits
        }
        ResolvedType::Generic(_) => {
            // TODO: handle blanket implementations. As a first approximation,
            //   we assume that if the type is generic, it implements all traits.
            return Ok(true);
        }
    }

    // We check if there is a trait implementation for this type in the crate where the trait
    // was defined.

    // Auto-traits (e.g. Send, Sync, etc.) always appear as implemented in the crate where
    // the implementer is defined.
    if trait_item.is_auto {
        return Ok(false);
    }

    for impl_id in &trait_item.implementations {
        let impl_item = trait_definition_crate.get_type_by_local_type_id(impl_id);
        let implementer = match &impl_item.inner {
            ItemEnum::Impl(impl_) => {
                if impl_.negative {
                    continue;
                }
                &impl_.for_
            }
            n => {
                dbg!(n);
                unreachable!()
            }
        };
        if is_equivalent(
            implementer,
            type_,
            krate_collection,
            &trait_definition_crate.core.package_id,
        ) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn is_equivalent(
    rustdoc_type: &Type,
    our_type: &ResolvedType,
    krate_collection: &CrateCollection,
    used_by_package_id: &PackageId,
) -> bool {
    match rustdoc_type {
        Type::ResolvedPath(p) => {
            let ResolvedType::ResolvedPath(our_path_type) = our_type else { return false; };
            let rustdoc_type_id = &p.id;
            let Ok((rustdoc_global_type_id, _)) = krate_collection
                .get_canonical_path_by_local_type_id(used_by_package_id, rustdoc_type_id)
                else {
                    tracing::trace!("Failed to look up {:?}", rustdoc_type_id);
                    return false;
                };
            let rustdoc_item = krate_collection.get_type_by_global_type_id(&rustdoc_global_type_id);
            // We want to see through type aliases
            if let ItemEnum::Typedef(typedef) = &rustdoc_item.inner {
                return is_equivalent(
                    &typedef.type_,
                    our_type,
                    krate_collection,
                    used_by_package_id,
                );
            }
            let Ok(rustdoc_type_path) = krate_collection
                .get_canonical_path_by_global_type_id(&rustdoc_global_type_id)
                else {
                    tracing::trace!("Failed to look up {:?}", rustdoc_global_type_id);
                    return false;
                };
            // This is a much weaker check than the one we should actually be performing.
            // We are only checking that the base path of the two types is the same, without inspecting
            // where bounds or which generic parameters are being used.
            if rustdoc_type_path == our_path_type.base_type {
                return true;
            }
        }
        Type::BorrowedRef {
            mutable,
            type_: inner_type,
            ..
        } => {
            if let ResolvedType::Reference(type_) = our_type {
                return type_.is_mutable == *mutable
                    && is_equivalent(
                        inner_type,
                        &type_.inner,
                        krate_collection,
                        used_by_package_id,
                    );
            }
        }
        Type::Tuple(rustdoc_tuple) => {
            if let ResolvedType::Tuple(our_tuple) = our_type {
                if our_tuple.elements.len() != rustdoc_tuple.len() {
                    return false;
                }
                for (our_tuple_element, rustdoc_tuple_element) in
                    our_tuple.elements.iter().zip(rustdoc_tuple.iter())
                {
                    if !is_equivalent(
                        rustdoc_tuple_element,
                        our_tuple_element,
                        krate_collection,
                        used_by_package_id,
                    ) {
                        return false;
                    }
                }
                return true;
            }
        }
        Type::Primitive(p) => {
            if let ResolvedType::ScalarPrimitive(our_primitive) = our_type {
                return our_primitive.as_str() == p;
            }
        }
        Type::Slice(s) => {
            if let ResolvedType::Slice(our_slice) = our_type {
                return is_equivalent(
                    s,
                    &our_slice.element_type,
                    krate_collection,
                    used_by_package_id,
                );
            }
        }
        n => {
            tracing::trace!("We don't handle {:?} yet", n);
        }
    }
    false
}

#[derive(Debug, Clone)]
pub(crate) struct MissingTraitImplementationError {
    pub type_: ResolvedType,
    pub trait_: ResolvedType,
}

impl std::error::Error for MissingTraitImplementationError {}

impl std::fmt::Display for MissingTraitImplementationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "`{:?}` doesn't implement the `{:?}` trait.",
            &self.type_, &self.trait_
        )
    }
}
