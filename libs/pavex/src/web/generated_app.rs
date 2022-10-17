use std::io::Write;
use std::path::{Path, PathBuf};

use proc_macro2::TokenStream;

#[derive(Clone)]
/// The manifest and the code for a generated application.
///
/// Built by [`App::codegen`](crate::web::App::codegen).
pub struct GeneratedApp {
    pub(crate) lib_rs: TokenStream,
    pub(crate) cargo_toml: cargo_manifest::Manifest,
}

impl GeneratedApp {
    /// Save the code and the manifest for the generated application to disk.
    /// The newly created library crate is also injected as a member into the current workspace.
    pub fn persist(mut self, directory: &Path) -> Result<(), anyhow::Error> {
        // `cargo metadata` seems to be the only reliable way of retrieving the path to
        // the root manifest of the current workspace for a Rust project.
        let package_graph = guppy::MetadataCommand::new().exec()?.build_graph()?;
        let workspace = package_graph.workspace();

        let directory = if directory.is_relative() {
            workspace.root().as_std_path().join(directory)
        } else {
            directory.to_path_buf()
        };

        Self::inject_app_into_workspace_members(&workspace, &directory)?;

        let lib_rs = prettyplease::unparse(&syn::parse2(self.lib_rs)?);

        if let Some(dependencies) = &mut self.cargo_toml.dependencies {
            for dependency in dependencies.values_mut() {
                if let cargo_manifest::Dependency::Detailed(detailed) = dependency {
                    if let Some(path) = &mut detailed.path {
                        let parsed_path = PathBuf::from(path.to_owned());
                        let relative_path = pathdiff::diff_paths(parsed_path, &directory).unwrap();
                        *path = relative_path.to_string_lossy().to_string();
                    }
                }
            }
        }
        let cargo_toml = toml::to_string(&self.cargo_toml)?;
        let cargo_toml_path = directory.join("Cargo.toml");
        let source_directory = directory.join("src");
        fs_err::create_dir_all(&source_directory)?;
        fs_err::write(source_directory.join("lib.rs"), lib_rs)?;
        fs_err::write(cargo_toml_path, cargo_toml)?;
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
        let mut root_manifest: toml::Value = toml::from_str(&root_manifest)?;

        let member_path = pathdiff::diff_paths(&generated_crate_directory, root_path)
            .unwrap()
            .to_string_lossy()
            .to_string();

        if root_manifest.get("workspace").is_none() {
            let root_manifest = root_manifest.as_table_mut().unwrap();
            let members = toml::Value::Array(vec![".".to_string().into(), member_path.into()]);
            let mut workspace = toml::value::Table::new();
            workspace.insert("members".into(), members);
            root_manifest.insert("workspace".into(), workspace.into());
        } else {
            let workspace = root_manifest
                .get_mut("workspace")
                .unwrap()
                .as_table_mut()
                .unwrap();
            if let Some(members) = workspace.get_mut("members") {
                if let Some(members) = members.as_array_mut() {
                    if !members
                        .iter().any(|m| m.as_str() == Some(&member_path))
                    {
                        members.push(member_path.into());
                    }
                }
            } else {
                let members = toml::Value::Array(vec![".".to_string().into(), member_path.into()]);
                workspace.insert("members".into(), members);
            }
        }
        let mut root_manifest_file = fs_err::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&root_manifest_path)?;
        root_manifest_file.write_all(toml::to_string(&root_manifest)?.as_bytes())?;
        Ok(())
    }
}
