use std::collections::{HashMap, HashSet};
use std::fmt::Formatter;

use bimap::BiHashMap;
use guppy::graph::PackageGraph;
use indexmap::IndexMap;
use rustdoc_types::{GenericParamDefKind, ItemEnum, Type};

use pavex_builder::Location;
use pavex_builder::RawCallableIdentifiers;

use crate::diagnostic;
use crate::diagnostic::CompilerDiagnostic;
use crate::diagnostic::{LocationExt, SourceSpanExt};
use crate::language::{Callable, ResolvedPath, ResolvedType};
use crate::rustdoc::CrateCollection;
use crate::web::constructors::Constructor;
use crate::web::resolvers::resolve_type;

/// It returns an error if `type_` does not implement the specified trait.
///
/// The trait path must be fully resolved: it should NOT point to a re-export
/// (e.g. `std::marker::Sync` won't work, you should use `core::marker::Sync`).
pub(crate) fn assert_trait_is_implemented(
    krate_collection: &CrateCollection,
    type_: &ResolvedType,
    expected_trait: &ResolvedType,
) -> Result<(), MissingTraitImplementationError> {
    if !implements_trait(krate_collection, type_, expected_trait) {
        Err(MissingTraitImplementationError {
            type_: type_.to_owned(),
            trait_: expected_trait.to_owned(),
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
    expected_trait: &ResolvedType,
) -> bool {
    let type_definition_crate = krate_collection.get_crate_by_package_id(&type_.package_id);
    let type_id = type_definition_crate
        .get_type_id_by_path(&type_.base_type)
        .unwrap();
    let type_item = krate_collection.get_type_by_global_type_id(type_id);
    if let ItemEnum::Typedef(typedef) = &type_item.inner {
        let mut generic_bindings = HashMap::new();
        for generic in &typedef.generics.params {
            match &generic.kind {
                GenericParamDefKind::Type {
                    default: Some(default),
                    ..
                } => {
                    let default = resolve_type(
                        &default,
                        &type_.package_id,
                        krate_collection,
                        &generic_bindings,
                    )
                    .unwrap();
                    generic_bindings.insert(generic.name.to_string(), default);
                }
                GenericParamDefKind::Type { default: None, .. }
                | GenericParamDefKind::Const { .. }
                | GenericParamDefKind::Lifetime { .. } => {
                    todo!("Generic parameters other than type parameters with a default value are not supported yet. I cannot handle:\n {:?}", generic)
                }
            }
        }
        let type_ = resolve_type(
            &typedef.type_,
            &type_.package_id,
            krate_collection,
            &generic_bindings,
        )
        .unwrap();
        return implements_trait(krate_collection, &type_, expected_trait);
    }

    // Due to Rust's orphan rule, a trait implementation for a type can live in two places:
    // - In the crate where the type was defined;
    // - In the crate where the trait was defined.
    // We start by checking if there is a trait implementation for this type in the crate where the
    // type was defined.
    let impls = match &type_item.inner {
        ItemEnum::Struct(s) => &s.impls,
        ItemEnum::Enum(e) => &e.impls,
        n => {
            dbg!(n);
            unreachable!()
        }
    };
    for impl_id in impls {
        let trait_id = match &type_definition_crate
            .get_type_by_local_type_id(impl_id)
            .inner
        {
            ItemEnum::Impl(impl_) => {
                if impl_.negative {
                    continue;
                }
                impl_.trait_.as_ref().map(|p| &p.id)
            }
            _ => unreachable!(),
        };
        if let Some(trait_id) = trait_id {
            if let Ok((_, trait_path)) =
                krate_collection.get_canonical_path_by_local_type_id(&type_.package_id, trait_id)
            {
                if trait_path == expected_trait.base_type {
                    return true;
                }
            }
        }
    }

    // We check if there is a trait implementation for this type in the crate where the trait
    // was defined.
    let trait_definition_crate =
        krate_collection.get_crate_by_package_id(&expected_trait.package_id);
    let trait_item_id = trait_definition_crate
        .get_type_id_by_path(&expected_trait.base_type)
        .unwrap();
    let trait_item = krate_collection.get_type_by_global_type_id(trait_item_id);
    let ItemEnum::Trait(trait_item) = &trait_item.inner else { unreachable!() };

    // Auto-traits (e.g. Send, Sync, etc.) always appear as implemented in the crate where
    // the implementer is defined.
    if trait_item.is_auto {
        return false;
    }

    'outer: for impl_id in &trait_item.implementations {
        let implementer = match &trait_definition_crate
            .get_type_by_local_type_id(impl_id)
            .inner
        {
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
        let implementer_id = match implementer {
            Type::ResolvedPath(p) => &p.id,
            Type::BorrowedRef { type_, .. } => match &**type_ {
                Type::ResolvedPath(p) => &p.id,
                _ => {
                    continue;
                }
            },
            _ => {
                continue;
            }
        };
        let Ok((mut implementer_id, _)) = krate_collection
            .get_canonical_path_by_local_type_id(&trait_item_id.package_id, implementer_id)
            else {
                continue;
            };
        // We want to see through type aliases
        'inner: loop {
            let implementer_item = krate_collection.get_type_by_global_type_id(&implementer_id);
            if let ItemEnum::Typedef(typedef) = &implementer_item.inner {
                let local_id = match &typedef.type_ {
                    Type::ResolvedPath(p) => &p.id,
                    Type::BorrowedRef { type_, .. } => match &**type_ {
                        Type::ResolvedPath(p) => &p.id,
                        n => {
                            dbg!("Not yet implemented: {:?}", n);
                            continue 'outer;
                        }
                    },
                    n => {
                        dbg!("Not yet implemented: {:?}", n);
                        break 'outer;
                    }
                };
                implementer_id = krate_collection
                    .get_canonical_path_by_local_type_id(&implementer_id.package_id, local_id)
                    .unwrap()
                    .0;
            } else {
                break 'inner;
            };
        }
        let implementer_path = krate_collection
            .get_canonical_path_by_global_type_id(&implementer_id)
            .unwrap();
        // This is a much weaker check than the one we should actually be performing.
        // We are only checking that the base path of the two types is the same, without inspecting
        // where bounds or which generic parameters are being used.
        if implementer_path == type_.base_type {
            return true;
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
            "`{:?}` does not implement the `{:?}` trait.",
            &self.type_, &self.trait_
        )
    }
}

impl MissingTraitImplementationError {
    pub(crate) fn into_diagnostic(
        mut self,
        constructors: &IndexMap<ResolvedType, Constructor>,
        constructor_callable_resolver: &BiHashMap<ResolvedPath, Callable>,
        resolved_paths2identifiers: &HashMap<ResolvedPath, HashSet<RawCallableIdentifiers>>,
        constructor_locations: &IndexMap<RawCallableIdentifiers, Location>,
        package_graph: &PackageGraph,
        help: Option<String>,
    ) -> Result<CompilerDiagnostic, miette::Error> {
        let constructor: &Constructor = match constructors.get(&self.type_) {
            Some(c) => c,
            None => {
                if self.type_.is_shared_reference {
                    self.type_.is_shared_reference = false;
                    &constructors[&self.type_]
                } else {
                    unreachable!()
                }
            }
        };
        let constructor_callable = match constructor {
            Constructor::Callable(c) => c,
            c => {
                dbg!(c);
                unreachable!()
            }
        };
        let constructor_path = constructor_callable_resolver
            .get_by_right(constructor_callable)
            .unwrap();
        let raw_identifier = resolved_paths2identifiers[constructor_path]
            .iter()
            .next()
            .unwrap();
        let location = &constructor_locations[raw_identifier];
        let source = location.source_file(&package_graph)?;
        let label = diagnostic::get_f_macro_invocation_span(&source, location)
            .map(|s| s.labeled("The constructor was registered here".into()));
        let diagnostic = CompilerDiagnostic::builder(source, self)
            .optional_label(label)
            .optional_help(help)
            .build();
        Ok(diagnostic)
    }
}
