use anyhow::{Context, anyhow};
use guppy::graph::PackageGraph;
use guppy::{PackageId, Version};
use indexmap::IndexSet;
use rustc_hash::FxHashMap;
use rustdoc_types::ExternalCrate;
use tracing_log_error::log_error;

use crate::TOOLCHAIN_CRATES;
use crate::utils::normalize_crate_name;
use crate::version_matcher::VersionMatcher;

/// The information used by [`Crate::compute_package_id_for_crate_id_with_hint`](super::Crate::compute_package_id_for_crate_id_with_hint)
/// to map a `crate_id` to a `package_id`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct CrateIdNeedle {
    pub(crate) crate_id: u32,
    pub(crate) maybe_dependent_crate_name: Option<String>,
}

fn get_external_crate_version(external_crate: &ExternalCrate) -> Option<Version> {
    if let Some(url) = &external_crate.html_root_url {
        url.trim_end_matches('/')
            .split('/')
            .next_back()
            .map(Version::parse)
            .and_then(|x| x.ok())
    } else {
        None
    }
}

/// Given a crate id for an external crate, return the corresponding [`PackageId`].
///
/// It panics if the provided crate id doesn't appear in the JSON documentation
/// for this crate—i.e. if it's not `0` or assigned to one of its transitive dependencies.
#[allow(clippy::disallowed_types)]
pub(crate) fn compute_package_id_for_crate_id(
    // The package id of the crate whose documentation we are currently processing.
    package_id: &PackageId,
    // The mapping from crate id to external crate object.
    external_crate_index: &FxHashMap<u32, ExternalCrate>,
    crate_id: u32,
    // There might be multiple crates in the dependency graph with the same name, causing
    // disambiguation issues.
    // To help out, you can specify `maybe_dependent`: the name of a crate that you think
    // depends on the crate you're trying to resolve.
    // This can narrow down the portion of the dependency graph that we need to search,
    // thus removing ambiguity.
    maybe_dependent_crate_name: Option<&str>,
    package_graph: &PackageGraph,
) -> Result<PackageId, anyhow::Error> {
    #[derive(Debug, Hash, Eq, PartialEq)]
    struct PackageLinkMetadata {
        id: PackageId,
        name: String,
        version: Version,
    }

    enum ResolvedDependency {
        Found(PackageId),
        Ambiguous(IndexSet<PackageLinkMetadata>),
        NotFound,
    }

    /// Find a transitive dependency of `search_root` given its name (and maybe the version).
    /// It only returns `Some` if the dependency can be identified without ambiguity.
    fn find_transitive_dependency(
        package_graph: &PackageGraph,
        search_root: &PackageId,
        name: &str,
        version: Option<&Version>,
    ) -> Option<PackageId> {
        match _find_transitive_dependency(package_graph, search_root, name, version) {
            Ok(ResolvedDependency::Found(id)) => Some(id),
            Ok(ResolvedDependency::Ambiguous(_) | ResolvedDependency::NotFound) => None,
            Err(e) => {
                log_error!(
                    *e,
                    level: tracing::Level::WARN,
                    external_crate.name = %name,
                    external_crate.version = ?version,
                    search_root = %search_root.repr(),
                    "Failed to find transitive dependency"
                );
                None
            }
        }
    }

    fn _find_transitive_dependency(
        package_graph: &PackageGraph,
        search_root: &PackageId,
        name: &str,
        version: Option<&Version>,
    ) -> Result<ResolvedDependency, anyhow::Error> {
        let transitive_dependencies = package_graph
            .query_forward([search_root])
            .with_context(|| {
                format!(
                    "`{}` doesn't appear in the package graph for the current workspace",
                    search_root.repr()
                )
            })?
            .resolve();
        let expected_link_name = normalize_crate_name(name);
        let package_candidates: IndexSet<_> = transitive_dependencies
            .links(guppy::graph::DependencyDirection::Forward)
            .filter(|link| normalize_crate_name(link.to().name()) == expected_link_name)
            .map(|link| {
                let l = link.to();
                PackageLinkMetadata {
                    id: l.id().to_owned(),
                    name: l.name().to_owned(),
                    version: l.version().clone(),
                }
            })
            .collect();
        if package_candidates.is_empty() {
            return Ok(ResolvedDependency::NotFound);
        }
        if package_candidates.len() == 1 {
            return Ok(ResolvedDependency::Found(
                package_candidates.into_iter().next().unwrap().id,
            ));
        }

        if let Some(expected_link_version) = version {
            let version_matcher = VersionMatcher::new(expected_link_version);
            let filtered_candidates: Vec<_> = package_candidates
                .iter()
                .filter(|l| version_matcher.matches(&l.version))
                .collect();
            if filtered_candidates.is_empty() {
                let candidates = package_candidates
                    .iter()
                    .map(|l| format!("- {}@{}", l.name, l.version))
                    .collect::<Vec<_>>()
                    .join("\n");
                anyhow::bail!(
                    "Searching for `{expected_link_name}` among the transitive dependencies \
                    of `{search_root}` led to multiple results:\n{candidates}\n\
                    When the version ({expected_link_version}) was added to the search filters, \
                    no results come up. Could the inferred version be incorrect?\n\
                    This can happen if `{expected_link_name}` is using `#![doc(html_root_url = \"..\")]` \
                    with a URL that points to the documentation for a different (older?) version of itself."
                )
            }
            if filtered_candidates.len() == 1 {
                return Ok(ResolvedDependency::Found(
                    filtered_candidates.first().unwrap().id.to_owned(),
                ));
            }
        }

        Ok(ResolvedDependency::Ambiguous(package_candidates))
    }

    if crate_id == 0 {
        return Ok(package_id.clone());
    }

    let external_crate = external_crate_index.get(&crate_id).ok_or_else(|| {
        anyhow!(
            "There is no external crate associated with id `{}` in the JSON documentation for `{}`",
            crate_id,
            package_id.repr()
        )
    })?;
    if TOOLCHAIN_CRATES.contains(&external_crate.name.as_str()) {
        return Ok(PackageId::new(external_crate.name.clone()));
    }
    let external_crate_version = get_external_crate_version(external_crate);
    let ambiguous_candidates = match _find_transitive_dependency(
        package_graph,
        package_id,
        &external_crate.name,
        external_crate_version.as_ref(),
    )? {
        ResolvedDependency::Found(id) => return Ok(id),
        ResolvedDependency::Ambiguous(candidates) => candidates,
        ResolvedDependency::NotFound => {
            return Err(anyhow!(
                "I could not find any crate named `{}` \
                among the dependencies (either direct or transitive) of {}",
                external_crate.name,
                package_id.repr()
            ));
        }
    };

    // We have multiple packages with the same name.
    // We need to disambiguate among them.
    if let Some(maybe_dependent_crate_name) = maybe_dependent_crate_name {
        let intermediate_crates: Vec<_> = external_crate_index
            .values()
            .filter(|c| c.name == maybe_dependent_crate_name)
            .collect();
        if intermediate_crates.len() == 1 {
            let intermediate_crate = intermediate_crates.first().unwrap();
            let intermediate_crate_version = get_external_crate_version(intermediate_crate);
            if let Some(intermediate_package_id) = find_transitive_dependency(
                package_graph,
                package_id,
                &intermediate_crate.name,
                intermediate_crate_version.as_ref(),
            ) && let Some(id) = find_transitive_dependency(
                package_graph,
                &intermediate_package_id,
                &external_crate.name,
                external_crate_version.as_ref(),
            ) {
                return Ok(id);
            }
        }
    }

    let candidates_list = ambiguous_candidates
        .iter()
        .map(|l| format!("- {} v{} ({})", l.name, l.version, l.id.repr()))
        .collect::<Vec<_>>()
        .join("\n");
    Err(anyhow!(
        "There are multiple packages named `{}` among the dependencies of {}:\n{}\n\
            In order to disambiguate among them, I need to know their versions.\n\
            Unfortunately, I couldn't extract the expected version for `{}` from HTML root URL included in the \
            JSON documentation for `{}`.\n\
            This due to a limitation in `rustdoc` itself: follow https://github.com/rust-lang/compiler-team/issues/622 \
            to track progress on this issue.",
        external_crate.name,
        package_id.repr(),
        candidates_list,
        external_crate.name,
        package_id.repr()
    ))
}
