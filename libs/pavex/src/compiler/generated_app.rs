use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use cargo_manifest::{Dependency, Edition};
use guppy::graph::PackageGraph;
use persist_if_changed::persist_if_changed;
use proc_macro2::TokenStream;
use serde::Serialize;
use toml_edit::ser::ValueSerializer;

#[derive(Clone)]
/// The manifest and the code for a generated application.
///
/// Built by [`App::codegen`](crate::compiler::App::codegen).
pub struct GeneratedApp {
    pub(crate) lib_rs: TokenStream,
    pub(crate) cargo_toml: GeneratedManifest,
    pub(crate) package_graph: PackageGraph,
}

#[derive(Clone, Debug)]
/// The fields that we *must* control in the manifest for the generated application.  
pub struct GeneratedManifest {
    /// The non-dev dependencies required by the generated code.
    pub dependencies: BTreeMap<String, Dependency>,
    /// Edition used by the generated code.
    pub edition: Edition,
}

impl GeneratedManifest {
    fn overwrite(&self, existing_manifest: &mut toml_edit::Document) {
        // Set dependencies
        existing_manifest["dependencies"] = toml_edit::Item::Table(
            self.dependencies
                .iter()
                .map(|(name, dependency)| {
                    let value = match dependency {
                        Dependency::Simple(s) => s.into(),
                        Dependency::Detailed(d) => {
                            Serialize::serialize(d, ValueSerializer::new()).unwrap()
                        }
                    };
                    (name.clone(), value)
                })
                .collect(),
        );
        // Set edition
        let edition_value = Serialize::serialize(&self.edition, ValueSerializer::new()).unwrap();
        existing_manifest["package"]["edition"] = toml_edit::Item::Value(edition_value);
    }
}

impl GeneratedApp {
    /// Save the code and the manifest for the generated application to disk.
    /// The newly created library crate is also injected as a member into the current workspace.
    #[tracing::instrument(skip_all, level=tracing::Level::INFO)]
    pub fn persist(self, directory: &Path) -> Result<(), anyhow::Error> {
        let Self {
            lib_rs,
            mut cargo_toml,
            package_graph,
        } = self;
        let workspace = package_graph.workspace();

        let pkg_directory = if directory.is_relative() {
            workspace.root().as_std_path().join(directory)
        } else {
            directory.to_path_buf()
        };

        Self::normalize_path_dependencies(&mut cargo_toml, &pkg_directory)?;
        Self::inject_app_into_workspace_members(&workspace, &pkg_directory)?;

        let source_directory = pkg_directory.join("src");
        fs_err::create_dir_all(&source_directory)?;
        Self::persist_manifest(&cargo_toml, &pkg_directory)?;

        let lib_rs = prettyplease::unparse(&syn::parse2(lib_rs)?);
        persist_if_changed(&source_directory.join("lib.rs"), lib_rs.as_bytes())?;

        Ok(())
    }

    /// All path dependencies should be relative to the root of the workspace in which
    /// the generated application is located.
    fn normalize_path_dependencies(
        cargo_toml: &mut GeneratedManifest,
        pkg_directory: &Path,
    ) -> Result<(), anyhow::Error> {
        for dependency in cargo_toml.dependencies.values_mut() {
            let Dependency::Detailed(detailed) = dependency else { continue; };
            if let Some(path) = &mut detailed.path {
                let parsed_path = PathBuf::from(path.to_owned());
                let relative_path = pathdiff::diff_paths(parsed_path, pkg_directory).unwrap();
                *path = relative_path.to_string_lossy().to_string();
            }
        }
        Ok(())
    }

    fn persist_manifest(
        cargo_toml: &GeneratedManifest,
        pkg_directory: &Path,
    ) -> Result<(), anyhow::Error> {
        let cargo_toml_path = pkg_directory.join("Cargo.toml");
        // If the manifest already exists, we need to modify it in place.
        let mut manifest = match fs_err::read_to_string(&cargo_toml_path) {
            Ok(manifest) => manifest.parse::<toml_edit::Document>()?,
            // Otherwise, we create a new one with the minimum required fields.
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    let mut manifest = toml_edit::Document::new();
                    let mut pkg_table = toml_edit::table();
                    pkg_table["name"] = toml_edit::value("application");
                    pkg_table["version"] = toml_edit::value("0.1.0");
                    manifest.as_table_mut().insert("package", pkg_table);
                    manifest
                } else {
                    return Err(e.into());
                }
            }
        };
        cargo_toml.overwrite(&mut manifest);

        let mut cargo_toml_file = fs_err::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&cargo_toml_path)?;
        cargo_toml_file.write_all(manifest.to_string().as_bytes())?;
        Ok(())
    }

    /// Inject the newly generated crate in the list of members for the current workspace.
    ///
    /// If the root manifest for the current project is not a workspace, it gets converted into one.
    fn inject_app_into_workspace_members(
        workspace: &guppy::graph::Workspace,
        generated_crate_directory: &Path,
    ) -> Result<(), anyhow::Error> {
        let root_path = workspace.root().as_std_path();
        let root_manifest_path = root_path.join("Cargo.toml");
        let root_manifest = fs_err::read_to_string(&root_manifest_path)?;
        let mut root_manifest = root_manifest.parse::<toml_edit::Document>()?;

        let member_path = pathdiff::diff_paths(generated_crate_directory, root_path)
            .unwrap()
            .to_string_lossy()
            .to_string();

        if root_manifest.get("workspace").is_none() {
            // Convert the root manifest into a workspace.
            let root_manifest = root_manifest.as_table_mut();

            let workspace = {
                let members = {
                    let mut members = toml_edit::Array::new();
                    members.push(".".to_string());
                    members.push(member_path.clone());
                    toml_edit::Value::Array(members)
                };
                let mut ws = toml_edit::Table::new();
                ws.insert("members".into(), toml_edit::Item::Value(members));
                toml_edit::Item::Table(ws)
            };
            root_manifest.insert("workspace".into(), workspace);
        } else {
            let workspace = root_manifest
                .get_mut("workspace")
                .unwrap()
                .as_table_mut()
                .unwrap();
            // The `members` key is optional—you can omit it if your workspace has
            // a single member, i.e. the package defined in the same manifest file.
            if let Some(members) = workspace.get_mut("members") {
                if let Some(members) = members.as_array_mut() {
                    if !members.iter().any(|m| m.as_str() == Some(&member_path)) {
                        members.push(member_path);
                    }
                }
            } else {
                let members = {
                    let mut members = toml_edit::Array::new();
                    members.push(".".to_string());
                    members.push(member_path.clone());
                    toml_edit::Value::Array(members)
                };
                workspace.insert("members".into(), toml_edit::Item::Value(members));
            }
        }
        let contents = root_manifest.to_string();
        persist_if_changed(&root_manifest_path, contents.as_bytes())?;
        Ok(())
    }
}
