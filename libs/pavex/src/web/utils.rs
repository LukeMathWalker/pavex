use guppy::graph::PackageGraph;

use pavex_builder::RawCallableIdentifiers;

use crate::language::{GenericArgument, ResolvedPath, ResolvedType};
use crate::rustdoc::CrateCollection;
use crate::web::resolvers::resolve_type_path;

/// Returns `true` if `t` is a `Result` type.
pub(crate) fn is_result(t: &ResolvedType) -> bool {
    let ResolvedType::ResolvedPath(t) = t else {
        return false;
    };
    t.base_type == ["core", "result", "Result"]
        || t.base_type == ["core", "prelude", "rust_2015", "v1", "Result"]
        || t.base_type == ["core", "prelude", "rust_2018", "v1", "Result"]
        || t.base_type == ["core", "prelude", "rust_2021", "v1", "Result"]
}

pub(crate) fn get_ok_variant(t: &ResolvedType) -> &ResolvedType {
    debug_assert!(is_result(t));
    let ResolvedType::ResolvedPath(t) = t else {
        unreachable!();
    };
    let GenericArgument::AssignedTypeParameter(t) = &t.generic_arguments[0] else {
        unreachable!()
    };
    t
}

pub(crate) fn get_err_variant(t: &ResolvedType) -> &ResolvedType {
    debug_assert!(is_result(t));
    let ResolvedType::ResolvedPath(t) = t else {
        unreachable!();
    };
    let GenericArgument::AssignedTypeParameter(t) = &t.generic_arguments[1] else {
        unreachable!()
    };
    t
}

/// Resolve a type path assuming that the crate is a dependency of `pavex_builder`.
pub(crate) fn process_framework_path(
    raw_path: &str,
    package_graph: &PackageGraph,
    krate_collection: &CrateCollection,
) -> ResolvedType {
    // We are relying on a little hack to anchor our search:
    // all framework types belong to crates that are direct dependencies of `pavex_builder`.
    // TODO: find a better way in the future.
    let identifiers =
        RawCallableIdentifiers::from_raw_parts(raw_path.into(), "pavex_builder".into());
    let path = ResolvedPath::parse(&identifiers, package_graph).unwrap();
    let (item, _) = path.find_rustdoc_items(krate_collection).unwrap();
    resolve_type_path(&path, &item.item, krate_collection).unwrap()
}
