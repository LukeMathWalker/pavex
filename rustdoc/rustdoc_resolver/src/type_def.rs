//! Converters for type definitions (structs, enums) and type aliases.

use rustdoc_types::{Item, ItemEnum};

use rustdoc_ext::RustdocKindExt;
use rustdoc_ir::{Generic, GenericArgument, GenericLifetimeParameter, PathType, Type};
use rustdoc_processor::CrateCollection;
use rustdoc_processor::indexing::CrateIndexer;
use rustdoc_processor::queries::Crate;

use crate::GenericBindings;
use crate::errors::{TypeResolutionError, UnsupportedConstGeneric};
use crate::resolve_type::{TypeAliasResolution, resolve_type};

/// Convert an enum or a struct definition from the JSON documentation
/// for a crate into our own representation for types.
///
/// # Panics
///
/// Panics if the item isn't of kind enum or struct.
pub fn rustdoc_new_type_def2type(
    item: &Item,
    krate: &Crate,
) -> Result<Type, UnsupportedConstGeneric> {
    assert!(
        matches!(&item.inner, ItemEnum::Struct(_) | ItemEnum::Enum(_)),
        "Unexpected item type, `{}`. Expected a struct or an enum.",
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
                GenericArgument::Lifetime(GenericLifetimeParameter::from_name(&arg.name))
            }
            rustdoc_types::GenericParamDefKind::Type { .. } => {
                // TODO: Use the default if available.
                GenericArgument::TypeParameter(Type::Generic(Generic {
                    name: arg.name.clone(),
                }))
            }
            rustdoc_types::GenericParamDefKind::Const { .. } => todo!(),
        };
        generic_arguments.push(arg);
    }

    Ok(Type::Path(PathType {
        package_id: krate.core.package_id.clone(),
        rustdoc_id: Some(item.id),
        base_type: path.into(),
        generic_arguments,
    }))
}

/// Convert a type alias definition from the JSON documentation
/// for a crate into our own representation for types.
///
/// # Panics
///
/// Panics if the item isn't a type alias.
pub fn rustdoc_type_alias2type<I: CrateIndexer>(
    item: &Item,
    krate: &Crate,
    krate_collection: &CrateCollection<I>,
    alias_resolution: TypeAliasResolution,
) -> Result<Type, TypeResolutionError> {
    let ItemEnum::TypeAlias(inner) = &item.inner else {
        unreachable!(
            "Unexpected item type, `{}`. Expected a a type alias.",
            item.inner.kind()
        )
    };
    let resolved = resolve_type(
        &inner.type_,
        &krate.core.package_id,
        krate_collection,
        &GenericBindings::default(),
        alias_resolution,
    )?;
    Ok(resolved)
}
