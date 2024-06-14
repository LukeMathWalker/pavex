use guppy::graph::PackageGraph;

use pavex_bp_schema::{RawIdentifiers, RegisteredAt};

use crate::compiler::resolvers::resolve_callable;
use crate::language::{Callable, GenericArgument, PathKind, ResolvedPath, ResolvedType};
use crate::rustdoc::CrateCollection;

use super::resolvers::resolve_type_path;

pub(crate) fn get_ok_variant(t: &ResolvedType) -> &ResolvedType {
    debug_assert!(t.is_result());
    let ResolvedType::ResolvedPath(t) = t else {
        unreachable!();
    };
    let GenericArgument::TypeParameter(t) = &t.generic_arguments[0] else {
        unreachable!()
    };
    t
}

pub(crate) fn get_err_variant(t: &ResolvedType) -> &ResolvedType {
    debug_assert!(t.is_result());
    let ResolvedType::ResolvedPath(t) = t else {
        unreachable!();
    };
    let GenericArgument::TypeParameter(t) = &t.generic_arguments[1] else {
        unreachable!()
    };
    t
}

/// Resolve a type path assuming that the crate is a dependency of `pavex`.
pub(crate) fn process_framework_path(
    raw_path: &str,
    package_graph: &PackageGraph,
    krate_collection: &CrateCollection,
) -> ResolvedType {
    let identifiers = RawIdentifiers::from_raw_parts(
        raw_path.into(),
        RegisteredAt {
            // We are relying on a little hack to anchor our search:
            // all framework types belong to crates that are direct dependencies of `pavex`.
            // TODO: find a better way in the future.
            crate_name: "pavex".to_owned(),
            module_path: "pavex".to_owned(),
        },
    );
    let path = ResolvedPath::parse(&identifiers, package_graph, PathKind::Type).unwrap();
    resolve_type_path(&path, krate_collection).unwrap()
}

/// Resolve a callable path assuming that the crate is a dependency of `pavex`.
pub(crate) fn process_framework_callable_path(
    raw_path: &str,
    package_graph: &PackageGraph,
    krate_collection: &CrateCollection,
) -> Callable {
    let identifiers = RawIdentifiers::from_raw_parts(
        raw_path.into(),
        RegisteredAt {
            // We are relying on a little hack to anchor our search:
            // all framework types belong to crates that are direct dependencies of `pavex`.
            // TODO: find a better way in the future.
            crate_name: "pavex".to_owned(),
            module_path: "pavex".to_owned(),
        },
    );
    let path = ResolvedPath::parse(&identifiers, package_graph, PathKind::Callable).unwrap();
    resolve_callable(krate_collection, &path).unwrap()
}

/// A generator of unique lifetime names.
#[derive(Debug, Clone)]
pub struct LifetimeGenerator {
    next: usize,
}

impl LifetimeGenerator {
    const ALPHABET: [char; 26] = [
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
        's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    ];

    pub fn new() -> Self {
        Self { next: 0 }
    }

    /// Generates a new lifetime name.
    pub fn next(&mut self) -> String {
        let next = self.next;
        self.next += 1;
        let round = next / Self::ALPHABET.len();
        let letter = Self::ALPHABET[next % Self::ALPHABET.len()];
        if round == 0 {
            format!("{letter}")
        } else {
            format!("{letter}{round}")
        }
    }
}
