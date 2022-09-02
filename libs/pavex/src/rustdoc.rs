use std::collections::HashMap;
use std::default::Default;
use std::fmt::{Display, Formatter};
use std::path::Path;

use anyhow::{anyhow, Context};
use guppy::graph::{PackageGraph, PackageMetadata, PackageSource};
use guppy::{PackageId, Version};
use rustdoc_types::ItemEnum;

use crate::language::ImportPath;

#[derive(Debug, thiserror::Error)]
#[error("I failed to retrieve information about the public types of a package in your workspace ('{package_spec}').")]
pub struct CannotGetCrateData {
    pub package_spec: String,
    #[source]
    pub source: anyhow::Error,
}

pub fn get_crate_data(
    root_folder: &Path,
    package_id_spec: &PackageIdSpecification,
) -> Result<rustdoc_types::Crate, CannotGetCrateData> {
    _get_crate_data(root_folder, package_id_spec).map_err(|e| CannotGetCrateData {
        package_spec: package_id_spec.to_string(),
        source: e,
    })
}

fn _get_crate_data(
    target_directory: &Path,
    package_id_spec: &PackageIdSpecification,
) -> Result<rustdoc_types::Crate, anyhow::Error> {
    // TODO: check that we have the nightly toolchain available beforehand in order to return
    // a good error.
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("+nightly")
        .arg("rustdoc")
        .arg("-q")
        .arg("-p")
        .arg(package_id_spec.to_string())
        .arg("--lib")
        .arg("--")
        .arg("--document-private-items")
        .arg("-Zunstable-options")
        .arg("-wjson");

    let status = cmd.status().context("Failed to run rustdoc")?;

    if !status.success() {
        anyhow::bail!("rustdoc exited with non-zero status code");
    }

    let json_path = target_directory
        .join("doc")
        .join(format!("{}.json", &package_id_spec.name));

    let json = fs_err::read_to_string(json_path).context("Failed to read rustdoc output")?;
    let krate = serde_json::from_str::<rustdoc_types::Crate>(&json)
        .context("Failed to deserialize rustdoc output")?;
    Ok(krate)
}

#[derive(Debug, Clone)]
pub struct CrateCollection(HashMap<String, Crate>, PackageGraph);

impl CrateCollection {
    pub fn new(package_graph: PackageGraph) -> Self {
        Self(Default::default(), package_graph)
    }

    pub fn get_or_compute_by_id(
        &mut self,
        package_id: &PackageId,
    ) -> Result<&Crate, CannotGetCrateData> {
        let package_metadata = self.1.metadata(package_id).expect("Unknown package ID");
        let package_spec = PackageIdSpecification::new(&package_metadata);
        if self.0.get(&package_spec.to_string()).is_none() {
            let krate = get_crate_data(
                self.1.workspace().target_directory().as_std_path(),
                &package_spec,
            )?;
            let krate = Crate::new(krate, package_id.to_owned());
            self.0.insert(package_spec.to_string(), krate);
        }
        Ok(&self.0[&package_spec.to_string()])
    }
}

/// A selector that follows the [package ID specification](https://doc.rust-lang.org/cargo/reference/pkgid-spec.html).
/// It is used as argument to the `-p`/`--package` flag in `cargo`'s commands.
pub struct PackageIdSpecification {
    source: Option<String>,
    name: String,
    version: Option<Version>,
}

impl PackageIdSpecification {
    pub fn new(metadata: &PackageMetadata) -> Self {
        let source = match metadata.source() {
            PackageSource::Workspace(source) | PackageSource::Path(source) => {
                let source = source.strip_prefix("path+").unwrap_or(source);
                if source.as_str().is_empty() {
                    source.to_string()
                } else {
                    let source = if source.is_relative() {
                        metadata.graph().workspace().root().join(source).to_string()
                    } else {
                        source.to_string()
                    };
                    format!("file:///{}", source)
                }
            }
            PackageSource::External(source) => {
                let s = if let Some(source) = source.strip_prefix("git+") {
                    source
                } else if let Some(source) = source.strip_prefix("registry+") {
                    source
                } else {
                    source
                };
                s.to_owned()
            }
        };
        let source = if source.is_empty() {
            None
        } else {
            Some(source)
        };
        let name = metadata.name().to_owned();
        let version = Some(metadata.version().to_owned());
        Self {
            source,
            name,
            version,
        }
    }
}

impl std::fmt::Display for PackageIdSpecification {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(source) = &self.source {
            write!(f, "{}#", source)?;
        }
        write!(f, "{}", &self.name)?;
        if let Some(version) = &self.version {
            write!(f, "@{}", version)?;
        }
        Ok(())
    }
}

/// Thin wrapper around [`rustdoc_types::Crate`] to:
/// - bundle a derived index (path <> id);
/// - provide query helpers with good error messages.
///
/// It also records the `PackageId` for the corresponding crate within the dependency tree
/// for the workspace it belongs to.
#[derive(Debug, Clone)]
pub struct Crate {
    package_id: PackageId,
    krate: rustdoc_types::Crate,
    path_index: HashMap<Vec<String>, rustdoc_types::Id>,
}

impl Crate {
    fn new(krate: rustdoc_types::Crate, package_id: PackageId) -> Self {
        let mut path_index: HashMap<_, _> = krate
            .paths
            .iter()
            .map(|(id, summary)| (summary.path.clone(), id.to_owned()))
            .collect();
        index_re_exports(&krate, vec![], &mut path_index, &krate.root);
        Self {
            package_id,
            krate,
            path_index,
        }
    }

    /// Given a crate id, return the corresponding external crate object.
    /// We also try to return the crate version, if we manage to parse it out of the crate HTML
    /// root URL.
    pub fn get_external_crate_name(
        &self,
        crate_id: u32,
    ) -> Option<(&rustdoc_types::ExternalCrate, Option<Version>)> {
        let external_crate = self.krate.external_crates.get(&crate_id);
        if let Some(external_crate) = external_crate {
            let version = if let Some(url) = &external_crate.html_root_url {
                url.trim_end_matches('/')
                    .split('/')
                    .last()
                    .map(guppy::Version::parse)
                    .and_then(|x| x.ok())
            } else {
                None
            };
            Some((external_crate, version))
        } else {
            None
        }
    }

    pub fn package_id(&self) -> &PackageId {
        &self.package_id
    }

    pub fn get_id_by_path(&self, path: &[String]) -> Result<&rustdoc_types::Id, UnknownTypePath> {
        self.path_index.get(path).ok_or_else(|| UnknownTypePath {
            type_path: path.to_owned(),
        })
    }

    pub fn get_summary_by_id(
        &self,
        id: &rustdoc_types::Id,
    ) -> Result<&rustdoc_types::ItemSummary, anyhow::Error> {
        self.krate.paths.get(id).ok_or_else(|| {
            anyhow!(
                "Failed to look up the type id `{}` in the rustdoc's path index. \
                This is likely to be a bug in rustdoc's JSON output.",
                id.0
            )
        })
    }

    pub fn get_type_by_id(&self, id: &rustdoc_types::Id) -> &rustdoc_types::Item {
        let type_ = self.krate.index.get(id);
        if type_.is_none() {
            panic!(
                "Failed to look up the type id `{}` in the rustdoc's index. \
                This is likely to be a bug in rustdoc's JSON output.",
                id.0
            )
        }
        type_.unwrap()
    }

    pub fn get_type_by_path(
        &self,
        path: &[String],
    ) -> Result<&rustdoc_types::Item, UnknownTypePath> {
        let id = self.get_id_by_path(path)?;
        Ok(self.get_type_by_id(id))
    }
}

fn index_re_exports<'a>(
    krate: &'a rustdoc_types::Crate,
    mut current_path: Vec<&'a str>,
    path_index: &mut HashMap<Vec<String>, rustdoc_types::Id>,
    current_item_id: &rustdoc_types::Id,
) {
    // TODO: handle visibility
    // TODO: the way we handle `current_path` is extremely wasteful,
    // we can likely reuse the same buffer throughout.
    let current_item = &krate.index[current_item_id];
    match &current_item.inner {
        ItemEnum::Module(m) => {
            let current_path_segment = current_item
                .name
                .as_deref()
                .expect("All 'module' items have a 'name' property");
            current_path.push(current_path_segment);
            for item_id in &m.items {
                index_re_exports(krate, current_path.clone(), path_index, item_id);
            }
        }
        ItemEnum::Import(i) => {
            if let Some(imported_id) = &i.id {
                match krate.index.get(imported_id) {
                    None => {
                        if let Some(imported_summary) = krate.paths.get(imported_id) {
                            debug_assert!(imported_summary.crate_id != 0);
                        } else {
                            panic!("The imported id is not listed in the index nor in the path section of rustdoc's JSON output")
                        }
                    }
                    Some(imported_item) => {
                        if let ItemEnum::Module(_) = imported_item.inner {
                            current_path.push(&i.name);
                        }
                        index_re_exports(krate, current_path.clone(), path_index, imported_id);
                    }
                }
            }
        }
        ItemEnum::Struct(_) => {
            let struct_name = current_item
                .name
                .as_deref()
                .expect("All 'struct' items have a 'name' property");
            current_path.push(struct_name);
            path_index.insert(
                current_path.into_iter().map(|s| s.to_string()).collect(),
                current_item_id.to_owned(),
            );
        }
        ItemEnum::Enum(_) => {
            let enum_name = current_item
                .name
                .as_deref()
                .expect("All 'enum' items have a 'name' property");
            current_path.push(enum_name);
            path_index.insert(
                current_path.into_iter().map(|s| s.to_string()).collect(),
                current_item_id.to_owned(),
            );
        }
        ItemEnum::Function(_) => {
            let function_name = current_item
                .name
                .as_deref()
                .expect("All 'function' items have a 'name' property");
            current_path.push(function_name);
            path_index.insert(
                current_path.into_iter().map(|s| s.to_string()).collect(),
                current_item_id.to_owned(),
            );
        }
        _ => {}
    }
}

#[derive(thiserror::Error, Debug)]
pub struct UnknownTypePath {
    pub type_path: ImportPath,
}

impl Display for UnknownTypePath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let type_path = self.type_path.join("::").replace(' ', "");
        let krate = self.type_path.first().unwrap();
        write!(
            f,
            "I could not find '{type_path}' in the auto-generated documentation for '{krate}'"
        )
    }
}
