use std::collections::BTreeSet;

use guppy::graph::DependencyDirection;
use itertools::Itertools;

#[test]
pub fn allowed_sys_crates() {
    let package_graph = guppy::MetadataCommand::new()
        .build_graph()
        .expect("failed to build package graph");

    let cli_ids: Vec<_> = package_graph
        .package_ids()
        .filter(|id| {
            let name = package_graph.metadata(id).unwrap().name();
            name == "pavex_cli" || name == "pavexc_cli"
        })
        .cloned()
        .collect();
    assert_eq!(cli_ids.len(), 2);

    let dependencies = package_graph
        .query_directed(&cli_ids, DependencyDirection::Forward)
        .expect("Failed to compute CLI dependencies");
    let mut native_dependencies = BTreeSet::new();
    for metadata in dependencies
        // Don't check dependencies that were brought in via the workspace hack crate.
        // Those don't affect the final release artifact.
        .resolve_with_fn(|_, link| link.resolved_name() != "px_workspace_hack")
        .packages(DependencyDirection::Forward)
    {
        if metadata.links().is_some() {
            native_dependencies.insert(metadata.id().to_owned());
        }
    }

    let allowed_sys_crates = BTreeSet::from([
        "windows-sys",
        "libsqlite3-sys",
        // It doesn't actually get pulled in, since it's behind an optional feature in `pavex`
        // that doesn't get activated by the CLI, but tracking that is messy.
        "aws-lc-rs",
        "aws-lc-sys",
        "aws-lc-fips-sys",
        "clang-sys",
        // We use the `static` feature of `lzma-sys` to avoid linking to the system LZMA library.
        "lzma-sys",
        // Various crates in the ecosystem are not proper "*-sys" crates, but they still
        // rely on the "link" field of `Cargo.toml` to leverage some of its properties.
        // We need to allow them as well.
        "prettyplease",
        "rayon-core",
        "ring",
        "wasm-bindgen-shared",
    ]);
    for native_dep in native_dependencies {
        let metadata = package_graph.metadata(&native_dep).unwrap();
        assert!(
            allowed_sys_crates.contains(metadata.name()),
            "`{}` brings in a native dependency to one of Pavex's CLIs, but it's not in the allowlist of sys crates: {}",
            metadata.name(),
            allowed_sys_crates.iter().join(", ")
        );
    }
}
