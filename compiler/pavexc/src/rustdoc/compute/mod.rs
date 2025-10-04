use std::io::{BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;

mod cache;
mod checksum;
mod format;
mod toolchain;

use ahash::{HashMap, HashMapExt};
pub(crate) use cache::{RustdocCacheKey, RustdocGlobalFsCache};

use anyhow::Context;
use format::check_format;
use guppy::graph::PackageGraph;
use guppy::{PackageId, Version};
use indexmap::IndexSet;
use itertools::Itertools as _;
use pavex_cli_shell::SHELL;
use serde::Deserialize;

use crate::rustdoc::TOOLCHAIN_CRATES;
use crate::rustdoc::package_id_spec::PackageIdSpecification;
use crate::rustdoc::utils::normalize_crate_name;

use self::toolchain::get_toolchain_crate_docs;

#[derive(Debug, thiserror::Error, Clone)]
#[error(
    "I failed to retrieve information about the public types of a package in your dependency tree ('{package_spec}')."
)]
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

/// Return the JSON documentation for one or more crates.
/// Crates are singled out, within the current workspace, using a [`PackageIdSpecification`].
///
/// The documentation is computed on the fly for crates that are local to the current workspace.
/// The documentation is retrieved via `rustup` for toolchain crates (e.g. `std`).
pub(super) fn compute_crate_docs<I>(
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
        // Some crates are not compiled as part of the dependency tree of the current workspace.
        // They are instead bundled as part of Rust's toolchain and automatically available for import
        // and usage in your crate: the standard library (`std`), `core` (a smaller subset of `std`
        // that doesn't require an allocator), `alloc` (a smaller subset of `std` that assumes you
        // can allocate).
        // Since those crates are pre-compiled (and somewhat special), we can't generate their
        // documentation on the fly. We assume that their JSON docs have been pre-computed and are
        // available for us to look at.
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

    let target_directory = package_graph.workspace().target_directory().as_std_path();
    for (i, chunk) in chunks.into_iter().enumerate() {
        if i > 0 {
            // All crates in the later chunks have at least another crate with the same name
            // in the dependency graph (otherwise they would have been in the first chunk).
            // We need to clean up the target directory to make sure that `cargo` doesn't
            // mistakenly believe that the JSON docs it generated for another version of
            // the crate can be reused.
            for (_, spec) in &chunk {
                let _ = fs_err::remove_file(json_doc_location(spec, target_directory));
            }
        } else {
            // If it's the first chunk, we need to check package by package.
            for (id, spec) in &chunk {
                let metadata = package_graph.metadata(id).unwrap();
                if package_graph
                    .packages()
                    .filter(|p| p.name() == metadata.name())
                    .count()
                    > 1
                {
                    let _ = fs_err::remove_file(json_doc_location(spec, target_directory));
                }
            }
        }

        if let Some(shell) = SHELL.get()
            && let Ok(mut shell) = shell.lock()
        {
            for (package_id, _) in chunk.iter() {
                let Ok(package_metadata) = package_graph.metadata(package_id) else {
                    continue;
                };
                let _ = shell.status(
                    "Documenting",
                    format!("{}@{}", package_metadata.name(), package_metadata.version()),
                );
            }
        }
        let timer = std::time::Instant::now();

        let outcome = _compute_crate_docs(
            toolchain_name,
            chunk.iter().map(|(_, spec)| spec),
            current_dir,
        );

        let duration = timer.elapsed();
        if let Some(shell) = SHELL.get()
            && let Ok(mut shell) = shell.lock()
        {
            for (package_id, _) in chunk.iter() {
                let Ok(package_metadata) = package_graph.metadata(package_id) else {
                    continue;
                };
                let _ = shell.status(
                    "Documented",
                    format!(
                        "{}@{} in {:.3} seconds",
                        package_metadata.name(),
                        package_metadata.version(),
                        duration.as_secs_f32()
                    ),
                );
            }
        }

        outcome?;

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

#[tracing::instrument(skip_all, fields(package_id_specs, cmd))]
fn _compute_crate_docs<'a, I>(
    toolchain_name: &str,
    package_id_specs: I,
    current_dir: &Path,
) -> Result<(), anyhow::Error>
where
    I: Iterator<Item = &'a PackageIdSpecification>,
{
    let package_id_specs: Vec<_> = package_id_specs.collect();
    tracing::Span::current().record("package_id_specs", package_id_specs.iter().join(", "));
    if package_id_specs.len() == 1 {
        _compute_single_crate_docs(toolchain_name, package_id_specs[0], current_dir)
    } else {
        _compute_multiple_crate_docs(toolchain_name, package_id_specs, current_dir)
    }
}

/// `cargo rustdoc` understands the structure of the expected output, so it won't
/// regenerate the JSON if it's already there and the crate hasn't changed.
/// Unfortunately, that's not the case for `cargo doc`, so we can't leverage the
/// same benefits in both the single and multi-crate case using the same command.
fn _compute_single_crate_docs(
    toolchain_name: &str,
    package_id_spec: &PackageIdSpecification,
    current_dir: &Path,
) -> Result<(), anyhow::Error> {
    let mut cmd = std::process::Command::new("rustup");
    cmd.arg("run")
        .current_dir(current_dir)
        .arg(toolchain_name)
        .arg("cargo")
        .arg("rustdoc")
        .arg("-q")
        .arg("--lib")
        .arg("-p")
        .arg(package_id_spec.to_string())
        .arg("-Zunstable-options")
        .arg("--output-format")
        .arg("json")
        .arg("--")
        .arg("--document-private-items")
        .arg("--document-hidden-items");

    tracing::Span::current().record("cmd", tracing::field::debug(&cmd));

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

fn _compute_multiple_crate_docs(
    toolchain_name: &str,
    package_id_specs: Vec<&PackageIdSpecification>,
    current_dir: &Path,
) -> Result<(), anyhow::Error> {
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
        cmd.arg("-p").arg(package_id_spec.to_string());
    }

    cmd.env("RUSTDOCFLAGS", rustdoc_options().join(" "));

    tracing::Span::current().record("cmd", tracing::field::debug(&cmd));

    let status = cmd
        .status()
        .with_context(|| format!("Failed to run `cargo doc`.\n{cmd:?}"))?;

    if !status.success() {
        anyhow::bail!(
            "An invocation of `cargo doc` exited with non-zero status code.\n{:?}",
            cmd
        );
    }
    Ok(())
}

/// The path to the JSON file generated by `rustdoc`.
fn json_doc_location(spec: &PackageIdSpecification, target_directory: &Path) -> PathBuf {
    target_directory
        .join("doc")
        .join(format!("{}.json", normalize_crate_name(&spec.name)))
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
    let json_path = json_doc_location(package_id_spec, target_directory);

    let span = tracing::trace_span!("Read and deserialize JSON output");
    let guard = span.enter();
    let file = fs_err::File::open(&json_path).context(
        "Failed to open the file containing the output of a `cargo rustdoc` invocation.",
    )?;
    let mut reader = BufReader::new(file);
    let mut deserializer = serde_json::Deserializer::from_reader(&mut reader);
    // The documentation for some crates (e.g. typenum) causes a "recursion limit exceeded" when
    // deserializing their docs using the default recursion limit.
    deserializer.disable_recursion_limit();
    let deserializer = serde_stacker::Deserializer::new(&mut deserializer);
    match rustdoc_types::Crate::deserialize(deserializer) {
        Ok(krate) => {
            drop(guard);
            Ok(krate)
        }
        Err(e) => {
            // Reset the reader to the beginning of the file.
            if reader.seek(SeekFrom::Start(0)).is_ok()
                && let Err(format_err) = check_format(reader)
            {
                return Err(format_err).with_context(|| {
                        format!(
                            "The JSON docs at `{}` are not in the expected format. \
                            Are you using the right version of the `nightly` toolchain, `{}`, to generate the JSON docs?",
                            json_path.display(), crate::DEFAULT_DOCS_TOOLCHAIN
                        )
                    });
            }

            Err(e).with_context(|| {
                format!(
                    "Failed to deserialize the output of a `cargo rustdoc` invocation (`{}`)",
                    json_path.display()
                )
            })
        }
    }
}
