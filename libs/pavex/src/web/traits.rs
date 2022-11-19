use std::collections::{HashMap, HashSet};
use std::fmt::Formatter;

use bimap::BiHashMap;
use guppy::graph::PackageGraph;
use indexmap::IndexMap;
use rustdoc_types::ItemEnum;

use pavex_builder::{Location, RawCallableIdentifiers};

use crate::language::{Callable, ResolvedPath, ResolvedType};
use crate::rustdoc::CrateCollection;
use crate::web::diagnostic::{CompilerDiagnosticBuilder, ParsedSourceFile, SourceSpanExt};
use crate::web::{diagnostic, CompilerDiagnostic};

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
    for impl_id in impls {
        let trait_id = match &krate.get_type_by_local_type_id(impl_id).inner {
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
                dbg!(trait_path);
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

impl MissingTraitImplementationError {
    pub(crate) fn into_diagnostic(
        mut self,
        constructor_callables: &IndexMap<ResolvedType, Callable>,
        constructor_callable_resolver: &BiHashMap<ResolvedPath, Callable>,
        resolved_paths2identifiers: &HashMap<ResolvedPath, HashSet<RawCallableIdentifiers>>,
        constructor_locations: &HashMap<RawCallableIdentifiers, Location>,
        package_graph: &PackageGraph,
    ) -> Result<CompilerDiagnostic, miette::Error> {
        let constructor_callable: &Callable = match constructor_callables.get(&self.type_) {
            Some(c) => c,
            None => {
                if self.type_.is_shared_reference {
                    self.type_.is_shared_reference = false;
                    &constructor_callables[&self.type_]
                } else {
                    unreachable!()
                }
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
        let source =
            ParsedSourceFile::new(location.file.as_str().into(), &package_graph.workspace())
                .map_err(miette::MietteError::IoError)?;
        let label =
            diagnostic::get_f_macro_invocation_span(&source.contents, &source.parsed, location)
                .map(|s| s.labeled("The singleton's constructor was registered here".into()));
        let diagnostic = CompilerDiagnosticBuilder::new(source, self)
            .optional_label(label)
            .build();
        Ok(diagnostic)
    }
}
