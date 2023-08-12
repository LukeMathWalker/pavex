use guppy::graph::PackageGraph;

use pavex::blueprint::reflection::RawCallableIdentifiers;

use crate::compiler::resolvers::resolve_type_path;
use crate::language::{GenericArgument, ResolvedPath, ResolvedType};
use crate::rustdoc::CrateCollection;

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
    // We are relying on a little hack to anchor our search:
    // all framework types belong to crates that are direct dependencies of `pavex`.
    // TODO: find a better way in the future.
    let identifiers =
        RawCallableIdentifiers::from_raw_parts(raw_path.into(), "pavex".into());
    let path = ResolvedPath::parse(&identifiers, package_graph).unwrap();
    let (item, _) = path.find_rustdoc_items(krate_collection).unwrap();
    resolve_type_path(&path, &item.item, krate_collection).unwrap()
}
