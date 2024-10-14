use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

mod cache;
mod checksum;
mod toolchain;

use ahash::{HashMap, HashMapExt};
pub(crate) use cache::{RustdocCacheKey, RustdocGlobalFsCache};

use anyhow::Context;
use guppy::graph::PackageGraph;
use guppy::{PackageId, Version};
use indexmap::IndexSet;
use serde::Deserialize;

use crate::rustdoc::package_id_spec::PackageIdSpecification;
use crate::rustdoc::utils::normalize_crate_name;
use crate::rustdoc::TOOLCHAIN_CRATES;

use self::toolchain::get_toolchain_crate_docs;

#[derive(Debug, thiserror::Error, Clone)]
#[error("I failed to retrieve information about the public types of a package in your workspace ('{package_spec}').")]
pub struct CannotGetCrateData {
    pub package_spec: String,
    #[source]
    pub source: Arc<anyhow::Error>,
}

fn format_optional_version(v: &Option<Version>) -> Option<tracing::field::DisplayValue<String>> {
    v.as_ref().map(|v| {
        use std::fmt::Write;
        let mut s = format!("v{}.{}.{}", v.major, v.minor, v.patch);
        if !v.pre.is_empty() {
            write!(&mut s, "-{}", v.pre).unwrap();
        }
        tracing::field::display(s)
    })
}

/// Return the JSON documentation for a crate.
/// The crate is singled out, within the current workspace, using a [`PackageIdSpecification`].
///
/// The documentation is computed on the fly for crates that are local to the current workspace.
/// The documentation is retrieved via `rustup` for toolchain crates (e.g. `std`).
///
/// `root_folder` is `cargo`'s target directory for the current workspace: that is where we are
/// going to look for the JSON files generated by `rustdoc`.
pub(super) fn compute_crate_docs(
    toolchain_name: &str,
    package_graph: &PackageGraph,
    package_id: &PackageId,
    current_dir: &Path,
) -> Result<rustdoc_types::Crate, CannotGetCrateData> {
    fn inner(
        package_graph: &PackageGraph,
        package_id: &PackageId,
        toolchain_name: &str,
        current_dir: &Path,
    ) -> Result<rustdoc_types::Crate, anyhow::Error> {
        // Some crates are not compiled as part of the dependency tree of the current workspace.
        // They are instead bundled as part of Rust's toolchain and automatically available for import
        // and usage in your crate: the standard library (`std`), `core` (a smaller subset of `std`
        // that doesn't require an allocator), `alloc` (a smaller subset of `std` that assumes you
        // can allocate).
        // Since those crates are pre-compiled (and somewhat special), we can't generate their
        // documentation on the fly. We assume that their JSON docs have been pre-computed and are
        // available for us to look at.
        if TOOLCHAIN_CRATES.contains(&package_id.repr()) {
            get_toolchain_crate_docs(package_id.repr(), toolchain_name)
        } else {
            let package_spec = PackageIdSpecification::from_package_id(package_id, package_graph)
                .map_err(|e| CannotGetCrateData {
                package_spec: package_id.to_string(),
                source: Arc::new(e),
            })?;
            _compute_crate_docs(toolchain_name, std::iter::once(&package_spec), current_dir)?;

            let target_directory = package_graph.workspace().target_directory().as_std_path();
            load_json_docs(target_directory, &package_spec)
        }
    }

    // We need to wrap the inner function in order to be able to return a `CannotGetCrateData`
    // error.
    // It's easier to do that here, rather than in the `inner` function, because we would need to
    // map the error of _every single fallible operation_.
    inner(package_graph, package_id, toolchain_name, current_dir).map_err(|e| CannotGetCrateData {
        package_spec: package_id.repr().to_owned(),
        source: Arc::new(e),
    })
}

/// A batch version of [`compute_crate_docs`].
///
/// This function is useful when you need to compute the documentation for multiple crates at once.
/// It is more efficient than calling [`compute_crate_docs`] multiple times, because `cargo doc`
/// (internally) can compute the documentation for multiple crates at once.
///
/// We can't obtain the same level of efficiency by calling `cargo rustdoc` multiple times, because
/// each invocation of `cargo rustdoc` will take an exclusive lock over the entire target directory,
/// causing the other invocations to queue, forcing us back into serial execution.
pub(super) fn batch_compute_crate_docs<I>(
    toolchain_name: &str,
    package_graph: &PackageGraph,
    package_ids: I,
    current_dir: &Path,
) -> Result<HashMap<PackageId, rustdoc_types::Crate>, anyhow::Error>
where
    I: Iterator<Item = PackageId>,
{
    let mut to_be_computed = vec![];
    let mut results = HashMap::new();
    for package_id in package_ids {
        if TOOLCHAIN_CRATES.contains(&package_id.repr()) {
            let krate = get_toolchain_crate_docs(package_id.repr(), toolchain_name)?;
            results.insert(package_id, krate);
            continue;
        }

        let package_spec = PackageIdSpecification::from_package_id(&package_id, package_graph)?;
        to_be_computed.push((package_id, package_spec));
    }

    if to_be_computed.is_empty() {
        return Ok(results);
    }

    // We need to chunk the crates into batches, because `cargo rustdoc` can only compute the
    // documentation for multiple crates at once if all the crate names are unique within the
    // batch.
    //
    // That's due to the output naming scheme of `rustdoc`: the output file is `{crate_name}.json`.
    // If we were to pass multiple crates with the same name to `cargo rustdoc`, the output file
    // would be overwritten multiple times, and we would only be left with the documentation for
    // the last crate.
    let chunks = {
        let mut chunks: Vec<Vec<(PackageId, PackageIdSpecification)>> = vec![];
        let mut chunk_id2names = HashMap::<usize, IndexSet<_>>::new();
        'outer: for (package_id, package_spec) in to_be_computed {
            for (index, chunk) in chunks.iter_mut().enumerate() {
                let chunk_names = chunk_id2names.get_mut(&index).unwrap();
                if chunk_names.insert(package_spec.name.clone()) {
                    // We haven't seen this crate name before.
                    chunk.push((package_id, package_spec));
                    continue 'outer;
                }
            }
            // We need a new chunk!
            let mut names = IndexSet::new();
            names.insert(package_spec.name.clone());
            chunk_id2names.insert(chunks.len(), names);
            chunks.push(vec![(package_id, package_spec)]);
        }
        chunks
    };

    for chunk in chunks {
        _compute_crate_docs(
            toolchain_name,
            chunk.iter().map(|(_, spec)| spec),
            current_dir,
        )?;
        let target_directory = package_graph.workspace().target_directory().as_std_path();

        // It takes a while to deserialize the JSON output of `cargo rustdoc`, so we parallelize
        // that part.
        use rayon::prelude::{IntoParallelIterator, ParallelIterator};
        for (package_id, krate) in chunk
            .into_par_iter()
            .map(|(package_id, package_spec)| {
                let krate = load_json_docs(target_directory, &package_spec);
                (package_id, krate)
            })
            .collect::<Vec<_>>()
        {
            results.insert(package_id, krate?);
        }
    }
    Ok(results)
}

/// Return the options to pass to `rustdoc` in order to generate JSON documentation.
///
/// We isolate this logic in a separate function in order to be able to refer to these
/// options from various places in the codebase and maintain a single source of truth.
///
/// In particular, they do affect our caching logic (see the `cache` module).
pub(super) fn rustdoc_options() -> [&'static str; 4] {
    [
        "--document-private-items",
        "-Zunstable-options",
        "-wjson",
        "--document-hidden-items",
    ]
}

#[tracing::instrument(skip_all, fields(package_id_specs))]
fn _compute_crate_docs<'a, I>(
    toolchain_name: &str,
    package_id_specs: I,
    current_dir: &Path,
) -> Result<(), anyhow::Error>
where
    I: Iterator<Item = &'a PackageIdSpecification>,
{
    let package_id_specs: Vec<_> = package_id_specs.map(|p| p.to_string()).collect();

    // TODO: check that we have the nightly toolchain available beforehand in order to return
    // a good error.
    let mut cmd = std::process::Command::new("rustup");
    cmd.arg("run")
        .current_dir(current_dir)
        .arg(toolchain_name)
        .arg("cargo")
        .arg("doc")
        .arg("--no-deps")
        .arg("-q")
        .arg("--lib");
    for package_id_spec in &package_id_specs {
        cmd.arg("-p").arg(package_id_spec);
    }
    tracing::Span::current().record("package_id_specs", &package_id_specs.join(", "));

    cmd.env("RUSTDOCFLAGS", rustdoc_options().join(" "));

    let status = cmd
        .status()
        .with_context(|| format!("Failed to run `cargo rustdoc`.\n{cmd:?}"))?;

    if !status.success() {
        anyhow::bail!(
            "An invocation of `cargo rustdoc` exited with non-zero status code.\n{:?}",
            cmd
        );
    }
    Ok(())
}

#[tracing::instrument(
    skip_all,
    fields(
        crate.name = package_id_spec.name,
        crate.version = format_optional_version(&package_id_spec.version),
        crate.source = package_id_spec.source
    )
)]
fn load_json_docs(
    target_directory: &Path,
    package_id_spec: &PackageIdSpecification,
) -> Result<rustdoc_types::Crate, anyhow::Error> {
    let json_path = target_directory.join("doc").join(format!(
        "{}.json",
        normalize_crate_name(&package_id_spec.name)
    ));

    let span = tracing::trace_span!("Read and deserialize JSON output");
    let guard = span.enter();
    let file = fs_err::File::open(&json_path).context(
        "Failed to open the file containing the output of a `cargo rustdoc` invocation.",
    )?;
    let reader = BufReader::new(file);
    let mut deserializer = serde_json::Deserializer::from_reader(reader);
    // The documention for some crates (e.g. typenum) causes a "recursion limit exceeded" when
    // deserializing their docs using the default recursion limit.
    deserializer.disable_recursion_limit();
    let deserializer = serde_stacker::Deserializer::new(&mut deserializer);
    let krate = rustdoc_types::Crate::deserialize(deserializer).with_context(|| {
        format!(
            "Failed to deserialize the output of a `cargo rustdoc` invocation (`{}`).",
            json_path.to_string_lossy()
        )
    })?;
    drop(guard);

    Ok(krate)
}
