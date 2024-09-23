use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output};

use ahash::{HashMap, HashSet};
use anyhow::Context;
use console::style;
use itertools::Itertools;
use libtest_mimic::{Arguments, Conclusion, Failed, Trial};
use pavexc::rustdoc::CrateCollection;
use pavexc::DEFAULT_DOCS_TOOLCHAIN;
use sha2::Digest;
use toml::toml;
use walkdir::WalkDir;

use persist_if_changed::{copy_if_changed, persist_if_changed};
pub use snapshot::print_changeset;

use crate::snapshot::SnapshotTest;

mod snapshot;

/// Return an iterator over the directories containing a UI test.
pub fn get_ui_test_directories(test_folder: &Path) -> impl Iterator<Item = PathBuf> {
    WalkDir::new(test_folder)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name() == "test_config.toml")
        .map(|entry| entry.path().parent().unwrap().to_path_buf())
}

pub fn get_test_name(tests_parent_folder: &Path, test_folder: &Path) -> String {
    let relative_path = test_folder.strip_prefix(&tests_parent_folder).unwrap();
    relative_path
        .components()
        .map(|c| {
            let Component::Normal(c) = c else {
                panic!("Expected a normal component")
            };
            c.to_string_lossy()
        })
        .join("::")
}

/// Create a test case for each folder in `definition_directory`.
///
/// Each test will get a separate runtime environment—a sub-folder of `runtime_directory`. The
/// same sub-folder is reused across multiple test runs to benefit from cargo's incremental compilation.
///
/// Custom configuration can be specified on a per-test basis by including a `test_config.toml` file
/// in the test folder. The available test options are detailed in `TestConfig`.
///
/// # cargo-nextest
///
/// Our custom test runner is built on top of `libtest_mimic`, which gives us
/// [compatibility out-of-the-box](https://nexte.st/book/custom-test-harnesses.html) with `cargo-nextest`.
pub fn run_tests(
    pavex_cli: PathBuf,
    pavexc_cli: PathBuf,
    definition_directory: PathBuf,
    runtime_directory: PathBuf,
) -> Result<Conclusion, anyhow::Error> {
    let arguments = libtest_mimic::Arguments::from_args();

    let mut test_name2test_data = BTreeMap::new();
    for entry in get_ui_test_directories(&definition_directory) {
        let test_name = get_test_name(&definition_directory, &entry);
        let relative_path = entry.strip_prefix(&definition_directory).unwrap();
        let test_data = TestData::new(
            &test_name,
            entry.clone(),
            runtime_directory.join(relative_path),
        )?;
        if is_filtered_out(&arguments, &test_name) || test_data.configuration.ignore {
            continue;
        }
        test_name2test_data.insert(test_name, test_data);
    }

    create_tests_dir(&runtime_directory, &test_name2test_data, &pavex_cli)?;

    let mut trials = Vec::new();
    if !arguments.list {
        warm_up_target_dir(&runtime_directory)?;
        warm_up_rustdoc_cache(&runtime_directory, &test_name2test_data)?;

        // TODO: We need to run them in parallel.
        for (name, data) in test_name2test_data {
            let (codegen_output, outcome) =
                code_generation_test(&runtime_directory, &data, &pavexc_cli);
            let is_success = outcome == CodegenTestOutcome::Success;
            let trial = outcome.into_trial(&name, &data.configuration, codegen_output.as_ref());
            trials.push(trial);

            // If the code generation test failed, we skip the follow-up tests.
            if !is_success {
                continue;
            }

            if let Some(codegen_output) = codegen_output {
                if let Some(trial) = code_generation_lints_test(&data, &name, &codegen_output) {
                    trials.push(trial);
                }
            };

            if data.configuration.expectations.codegen == ExpectedOutcome::Fail {
                continue;
            }

            let trial = code_generation_diagnostics_test(&name, &data);
            trials.push(trial);

            let trial = application_code_test(&name, &data);
            trials.push(trial);

            let trial_name = format!("{}::app_code_compiles", name);
            match application_code_compilation_test(&data) {
                Ok(_) => {
                    let trial = Trial::test(trial_name, || Ok(()));
                    trials.push(trial);
                }
                Err(err) => {
                    let msg = format!("{err:?}");
                    let trial = Trial::test(trial_name, move || Err(Failed::from(msg)));
                    trials.push(trial);

                    // We skip the follow-up tests if the application code compilation test failed.
                    continue;
                }
            }

            if data.should_run_tests() {
                let trial_name = format!("{}::app_integration_tests", name);
                match application_integration_test(&data) {
                    Ok(_) => {
                        let trial = Trial::test(trial_name, || Ok(()));
                        trials.push(trial);
                    }
                    Err(err) => {
                        let msg = format!("{err:?}");
                        let trial = Trial::test(trial_name, move || Err(Failed::from(msg)));
                        trials.push(trial);
                    }
                }
            }
        }
    }
    Ok(libtest_mimic::run(&arguments, trials))
}

// Inlined from `libtest_mimic` to further control the test execution.
// We ignore test kind since we don't have benches in our UI test suite.
fn is_filtered_out(args: &Arguments, test_name: &str) -> bool {
    // If a filter was specified, apply this
    if let Some(filter) = &args.filter {
        match args.exact {
            true if test_name != filter => return true,
            false if !test_name.contains(filter) => return true,
            _ => {}
        };
    }

    // If any skip pattern were specified, test for all patterns.
    for skip_filter in &args.skip {
        match args.exact {
            true if test_name == skip_filter => return true,
            false if test_name.contains(skip_filter) => return true,
            _ => {}
        }
    }

    false
}

/// Compile all binary targets of the type `app_*`.
/// This ensures that all dependencies have been compiled, speeding up further operations,
/// as well as preparing the binaries that each test will invoke.
fn warm_up_target_dir(runtime_directory: &Path) -> Result<(), anyhow::Error> {
    println!("Creating a workspace-hack crate to unify dependencies");

    // Clean up pre-existing workspace_hack, since `cargo hakari init` will fail
    // if it already exists.
    // let _ = fs_err::remove_dir_all(runtime_directory.join("workspace_hack"));
    // let _ = fs_err::remove_file(runtime_directory.join(".config").join("hakari.toml"));

    // let mut cmd = Command::new("cargo");
    // cmd.arg("hakari")
    //     .arg("init")
    //     .arg("-y")
    //     .arg("workspace_hack")
    //     .current_dir(runtime_directory)
    //     .stdout(std::process::Stdio::inherit())
    //     .stderr(std::process::Stdio::inherit());
    // let status = cmd.status()?;
    // if !status.success() {
    //     anyhow::bail!("Failed to create workspace_hack crate");
    // }

    // let mut cmd = Command::new("cargo");
    // cmd.arg("hakari")
    //     .arg("generate")
    //     .current_dir(runtime_directory)
    //     .stdout(std::process::Stdio::inherit())
    //     .stderr(std::process::Stdio::inherit());
    // let status = cmd.status()?;
    // if !status.success() {
    //     anyhow::bail!("Failed to generate workspace_hack crate");
    // }

    // let mut cmd = Command::new("cargo");
    // cmd.arg("hakari")
    //     .arg("manage-deps")
    //     .arg("-y")
    //     .current_dir(runtime_directory)
    //     .stdout(std::process::Stdio::inherit())
    //     .stderr(std::process::Stdio::inherit());
    // let status = cmd.status()?;
    // if !status.success() {
    //     anyhow::bail!("Failed to manage workspace_hack dependencies");
    // }

    let timer = std::time::Instant::now();
    println!("Warming up the target directory");
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("--bins")
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .current_dir(runtime_directory);
    let status = cmd.status()?;
    if !status.success() {
        anyhow::bail!("Failed to compile the test binaries");
    }
    println!(
        "Warmed up the target directory in {} seconds",
        timer.elapsed().as_secs()
    );

    Ok(())
}

/// Ensure all code generators won't have to invoke `rustdoc` to generate JSON documentation
/// by pre-computing the JSON documentation for all relevant crates.
fn warm_up_rustdoc_cache(
    runtime_directory: &Path,
    test_name2test_data: &BTreeMap<String, TestData>,
) -> Result<(), anyhow::Error> {
    let timer = std::time::Instant::now();
    // We want to ensure that all invocations of `pavexc generate` hit the cache
    // thus avoiding the need to invoke `rustdoc` and acquire a contentious
    // lock over the target directory.
    println!("Pre-computing JSON documentation for relevant crates");
    let crate_collection = CrateCollection::new(
        runtime_directory.to_string_lossy().into_owned(),
        DEFAULT_DOCS_TOOLCHAIN.to_owned(),
        runtime_directory.to_path_buf(),
    )?;
    let mut crates = test_name2test_data
        .values()
        .map(|data| format!("app_{}", data.name_hash))
        // TODO: we need to include locally-defined dependency crates as well
        .collect::<HashSet<_>>();
    // Hand-picked crates that we know we're going to build docs for.
    crates.insert("pavex".into());
    crates.insert("tracing".into());
    crates.insert("equivalent".into());
    crates.insert("ppv-lite86".into());
    crates.insert("hashbrown".into());
    crates.insert("typenum".into());
    let package_ids = crate_collection
        .package_graph()
        .packages()
        .filter(|p| crates.contains(p.name()))
        .map(|p| p.id().to_owned());
    crate_collection
        .batch_compute_crates(package_ids)
        .context("Failed to warm rustdoc JSON cache")?;
    println!(
        "Pre-computed JSON documentation in {} seconds",
        timer.elapsed().as_secs()
    );

    Ok(())
}

fn create_tests_dir(
    runtime_directory: &Path,
    test_name2test_data: &BTreeMap<String, TestData>,
    pavex_cli: &Path,
) -> Result<(), anyhow::Error> {
    let timer = std::time::Instant::now();
    println!("Seeding the filesystem");
    fs_err::create_dir_all(&runtime_directory)
        .context("Failed to create runtime directory for UI tests")?;

    // Create a `Cargo.toml` to define a workspace,
    // where each UI test is a workspace member
    let cargo_toml_path = runtime_directory.join("Cargo.toml");
    let mut cargo_toml = String::new();
    writeln!(&mut cargo_toml, "[workspace]\nmembers = [").unwrap();
    for test_data in test_name2test_data.values() {
        for member in test_data.workspace_members() {
            let relative_path = member.strip_prefix(&runtime_directory).unwrap();
            writeln!(cargo_toml, "  \"{}\",", relative_path.display()).unwrap();
        }
    }
    writeln!(&mut cargo_toml, "]").unwrap();
    writeln!(&mut cargo_toml, "resolver = \"2\"").unwrap();
    writeln!(&mut cargo_toml, "[workspace.dependencies]").unwrap();
    writeln!(&mut cargo_toml, "pavex = {{ path = \"../pavex\" }}").unwrap();
    writeln!(
        &mut cargo_toml,
        "pavex_cli_client = {{ path = \"../pavex_cli_client\" }}"
    )
    .unwrap();
    writeln!(
        &mut cargo_toml,
        "workspace_hack = {{ path = \"workspace_hack\" }}"
    )
    .unwrap();
    writeln!(&mut cargo_toml, "tokio = \"1\"").unwrap();
    writeln!(&mut cargo_toml, "reqwest = \"0.12\"").unwrap();

    persist_if_changed(&cargo_toml_path, cargo_toml.as_bytes())?;

    // Create a manifest for each UI test
    // Each UI test is composed of multiple crates, therefore we nest
    // everything under a test-specific directory to avoid name
    // clashes and confusion across tests
    for test_data in test_name2test_data.values() {
        test_data.seed_test_filesystem(pavex_cli)?;
    }

    println!(
        "Seeded the filesystem in {:?}ms",
        timer.elapsed().as_millis()
    );

    Ok(())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
/// Configuration values that can be specified next to the test data to influence how it's going
/// to be executed.
struct TestConfig {
    /// A short description explaining what the test is about, primarily for documentation purposes.
    /// It will be shown in the terminal if the test fails.
    description: String,
    /// Define what we expect to see when running the tests (e.g. should code generation succeed or fail?).
    #[serde(default)]
    expectations: TestExpectations,
    /// Ephemeral crates that should be generated as part of the test setup in order to be
    /// used as dependencies of the main crate under test.
    #[serde(default)]
    ephemeral_dependencies: HashMap<String, EphemeralDependency>,
    /// Crates that should be listed as dependencies of the package under the test, in addition to
    /// Pavex itself.
    #[serde(default)]
    dependencies: toml::value::Table,
    /// Crates that should be listed as dev dependencies of the test package.
    #[serde(default, rename = "dev-dependencies")]
    dev_dependencies: toml::value::Table,
    /// Ignore the test if set to `true`.
    #[serde(default)]
    ignore: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
struct EphemeralDependency {
    #[serde(default)]
    /// The name of the package in the generated `Cargo.toml`.
    /// If not specified, the corresponding key in [`TestConfig::ephemeral_dependencies`] will be used.
    package: Option<String>,
    /// The path to the file that should be used as `lib.rs` in the generated library crate.
    path: PathBuf,
    /// Crates that should be listed as dependencies of generated library crate.
    #[serde(default)]
    dependencies: toml::value::Table,
    #[serde(default = "default_ephemeral_version")]
    /// The version of the package in the generated `Cargo.toml`.
    /// If not specified, it defaults to `0.1.0`.
    version: String,
}

fn default_ephemeral_version() -> String {
    "0.1.0".to_string()
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
struct TestExpectations {
    /// By default, we expect code generation (i.e. `app.build()`) to succeed.
    /// If set to `fail`, the test runner will look for a snapshot of the expected failure message
    /// returned by Pavex to the user.
    #[serde(default = "ExpectedOutcome::pass")]
    codegen: ExpectedOutcome,
    #[serde(default = "ExpectedOutcome::pass")]
    lints: ExpectedOutcome,
}

impl Default for TestExpectations {
    fn default() -> Self {
        Self {
            codegen: ExpectedOutcome::Pass,
            lints: ExpectedOutcome::Pass,
        }
    }
}

#[derive(serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ExpectedOutcome {
    Pass,
    Fail,
}

impl ExpectedOutcome {
    fn pass() -> ExpectedOutcome {
        ExpectedOutcome::Pass
    }
}

/// Auxiliary data attached to each test definition for convenient retrieval.
/// It's used in [`run_test`].
struct TestData {
    name_hash: String,
    definition_directory: PathBuf,
    runtime_directory: PathBuf,
    configuration: TestConfig,
    has_tests: bool,
}

impl TestData {
    fn new(
        test_name: &str,
        definition_directory: PathBuf,
        runtime_directory: PathBuf,
    ) -> Result<Self, anyhow::Error> {
        let name_hash = {
            let mut hasher = sha2::Sha256::default();
            <_ as sha2::Digest>::update(&mut hasher, test_name.as_bytes());
            let full_hash = hasher.finalize();
            // Get the first 8 hex characters of the hash, they should be enough to identify the test
            let mut hash = String::new();
            for byte in full_hash.iter().take(4) {
                write!(&mut hash, "{:02x}", byte).unwrap();
            }
            hash
        };
        let configuration = Self::load_configuration(&definition_directory)?;
        let integration_test_file = definition_directory.join("test.rs");
        let has_tests = integration_test_file.exists();
        Ok(Self {
            name_hash,
            definition_directory,
            configuration,
            runtime_directory,
            has_tests,
        })
    }

    fn workspace_members(&self) -> Vec<PathBuf> {
        let mut members = vec![
            self.blueprint_directory().to_path_buf(),
            self.generated_app_directory(),
        ];
        if let Some(dir) = self.integration_test_directory() {
            members.push(dir.to_path_buf());
        }
        members
    }

    fn load_configuration(definition_directory: &Path) -> Result<TestConfig, anyhow::Error> {
        let path = definition_directory.join("test_config.toml");
        let test_config = fs_err::read_to_string(&path).context(
            "All UI tests must have an associated `test_config.toml` file with, \
                    at the very least, a `description` field explaining what the test is trying \
                    to verify.",
        )?;
        toml::from_str(&test_config).with_context(|| {
            format!(
                "Failed to deserialize {:?}. Check the file against the expected schema!",
                &path
            )
        })
    }

    /// The directory containing the source code of all ephemeral dependencies.
    ///
    /// We don't want to list ephemeral dependencies as members of the workspace of the project under test
    /// in order to be able to have multiple versions of the same crate as dependencies of the project under test.
    /// That would be forbidden by `cargo` if they were listed as members of the same workspace.
    fn ephemeral_deps_runtime_directory(&self) -> PathBuf {
        self.runtime_directory.join("ephemeral_deps")
    }

    fn blueprint_directory(&self) -> &Path {
        &self.runtime_directory
    }

    fn expectations_directory(&self) -> PathBuf {
        self.definition_directory.join("expectations")
    }

    fn generated_app_directory(&self) -> PathBuf {
        self.runtime_directory.join("generated_app")
    }

    fn blueprint_crate_name(&self) -> String {
        format!("app_{}", self.name_hash)
    }

    fn integration_test_directory(&self) -> Option<PathBuf> {
        self.has_tests
            .then(|| self.runtime_directory.join("integration"))
    }

    /// Populate the runtime test folder using the directives and the files in the test
    /// definition folder.
    fn seed_test_filesystem(&self, pavex_cli: &Path) -> Result<(), anyhow::Error> {
        fs_err::create_dir_all(&self.runtime_directory)
            .context("Failed to create runtime directory for UI test")?;
        let source_directory = self.runtime_directory.join("src");
        fs_err::create_dir_all(&source_directory).context(
            "Failed to create the runtime directory for the project under test when setting up the test runtime environment",
        )?;
        copy_if_changed(
            &self.definition_directory.join("lib.rs"),
            &source_directory.join("lib.rs"),
        )?;

        let deps_subdir = self.ephemeral_deps_runtime_directory();
        fs_err::create_dir_all(&source_directory).context(
            "Failed to create the runtime directory for ephemeral dependencies when setting up the test runtime environment",
        )?;

        for (dependency_name, dependency_config) in &self.configuration.ephemeral_dependencies {
            let dep_runtime_directory = deps_subdir.join(dependency_name);
            let package_name = dependency_config
                .package
                .clone()
                .unwrap_or(dependency_name.to_owned());
            let dep_source_directory = dep_runtime_directory.join("src");
            fs_err::create_dir_all(&dep_source_directory).context(
                "Failed to create the source directory for an ephemeral dependency when setting up the test runtime environment",
            )?;

            copy_if_changed(
                &self.definition_directory.join(&dependency_config.path),
                &dep_source_directory.join("lib.rs"),
            )?;

            let mut cargo_toml = toml! {
                [package]
                name = "dummy"
                version = "0.1.0"
                edition = "2021"

                [lints.rust]
                unexpected_cfgs = { level = "allow", check-cfg = ["cfg(pavex_ide_hint)"] }

                [dependencies]
                pavex = { workspace = true }
            };
            cargo_toml["package"]["name"] = format!("{package_name}_{}", self.name_hash).into();
            cargo_toml["package"]["version"] = dependency_config.version.clone().into();
            let deps = cargo_toml
                .get_mut("dependencies")
                .unwrap()
                .as_table_mut()
                .unwrap();
            deps.extend(dependency_config.dependencies.clone());

            persist_if_changed(
                &dep_runtime_directory.join("Cargo.toml"),
                toml::to_string(&cargo_toml)?.as_bytes(),
            )?;
        }

        if let Some(integration_test_directory) = self.integration_test_directory() {
            let integration_test_file = self.definition_directory.join("test.rs");
            let integration_test_src_directory = integration_test_directory.join("src");
            let integration_test_test_directory = integration_test_directory.join("tests");
            fs_err::create_dir_all(&integration_test_src_directory).context(
                "Failed to create the runtime directory for integration tests when setting up the test runtime environment",
            )?;
            fs_err::create_dir_all(&integration_test_test_directory).context(
                "Failed to create the runtime directory for integration tests when setting up the test runtime environment",
            )?;
            copy_if_changed(
                &integration_test_file,
                &integration_test_test_directory.join("run.rs"),
            )?;
            persist_if_changed(&integration_test_src_directory.join("lib.rs"), b"")?;

            let mut cargo_toml = toml! {
                [package]
                name = "dummy"
                version = "0.1.0"
                edition = "2021"

                [dependencies]
                application = { path = "../generated_app" }
                app = { path = ".." }

                [dev-dependencies]
                tokio = { workspace = true, features = ["full"] }
                reqwest = { workspace = true }
                pavex = { workspace = true }
            };
            cargo_toml["package"]["name"] = format!("integration_{}", self.name_hash).into();
            cargo_toml["dependencies"]["application"]
                .as_table_mut()
                .unwrap()
                .insert(
                    "package".into(),
                    format!("application_{}", self.name_hash).into(),
                );
            cargo_toml["dependencies"]["app"]
                .as_table_mut()
                .unwrap()
                .insert("package".into(), format!("app_{}", self.name_hash).into());

            let dev_deps = cargo_toml
                .get_mut("dev-dependencies")
                .unwrap()
                .as_table_mut()
                .unwrap();
            dev_deps.extend(self.configuration.dev_dependencies.clone());

            persist_if_changed(
                &integration_test_directory.join("Cargo.toml"),
                toml::to_string(&cargo_toml)?.as_bytes(),
            )?;
        }

        // Generated application crate, ahead of code generation.
        {
            let application_dir = self.generated_app_directory();
            let application_src_dir = application_dir.join("src");
            fs_err::create_dir_all(&application_src_dir).context(
                "Failed to create the runtime directory for the generated application when setting up the test runtime environment",
            )?;
            persist_if_changed(&application_src_dir.join("lib.rs"), b"")?;

            let mut cargo_toml = toml! {
                [package]
                name = "dummy"
                version = "0.1.0"
                edition = "2021"

                [package.metadata.px.generate]
                generator_type = "cargo_workspace_binary"
                generator_name = "app"
            };
            cargo_toml["package"]["name"] = format!("application_{}", self.name_hash).into();
            persist_if_changed(
                &application_dir.join("Cargo.toml"),
                toml::to_string(&cargo_toml)?.as_bytes(),
            )?;
        }

        let mut cargo_toml = toml! {
            [package]
            name = "dummy"
            version = "0.1.0"
            edition = "2021"

            [lints.rust]
            unexpected_cfgs = { level = "allow", check-cfg = ["cfg(pavex_ide_hint)"] }

            [dependencies]
            pavex = { workspace = true }
            pavex_cli_client = { workspace = true }
            workspace_hack = { workspace = true }
        };
        cargo_toml["package"]["name"] = format!("app_{}", self.name_hash).into();
        let deps = cargo_toml
            .get_mut("dependencies")
            .unwrap()
            .as_table_mut()
            .unwrap();
        deps.extend(self.configuration.dependencies.clone());
        let ephemeral_dependencies =
            self.configuration
                .ephemeral_dependencies
                .iter()
                .map(|(key, config)| {
                    let mut value = toml::value::Table::new();
                    value.insert("path".into(), format!("ephemeral_deps/{key}").into());
                    let package_name = if let Some(package_name) = config.package.as_ref() {
                        package_name.to_owned()
                    } else {
                        key.to_owned()
                    };
                    value.insert(
                        "package".into(),
                        format!("{package_name}_{}", self.name_hash).into(),
                    );
                    (key.to_owned(), toml::Value::Table(value))
                });
        deps.extend(ephemeral_dependencies);

        persist_if_changed(
            &self.runtime_directory.join("Cargo.toml"),
            toml::to_string(&cargo_toml)?.as_bytes(),
        )?;

        let cargo_config = toml! {
            [build]
            incremental = false

            [target.x86_64-pc-windows-msvc]
            rustflags = ["-C", "link-arg=-fuse-ld=lld"]
            [target.x86_64-pc-windows-gnu]
            rustflags = ["-C", "link-arg=-fuse-ld=lld"]
            [target.x86_64-unknown-linux-gnu]
            rustflags = ["-C", "linker=clang", "-C", "link-arg=-fuse-ld=lld"]
            [target.x86_64-apple-darwin]
            rustflags = ["-C", "link-arg=-fuse-ld=/usr/local/opt/llvm/bin/ld64.lld"]
            [target.aarch64-apple-darwin]
            rustflags = ["-C", "link-arg=-fuse-ld=/opt/homebrew/opt/llvm/bin/ld64.lld"]
        };
        let dot_cargo_folder = self.runtime_directory.join(".cargo");
        fs_err::create_dir_all(&dot_cargo_folder)?;
        persist_if_changed(
            &dot_cargo_folder.join("config.toml"),
            toml::to_string(&cargo_config)?.as_bytes(),
        )?;

        let main_rs = format!(
            r##"use app_{}::blueprint;
use pavex_cli_client::{{Client, config::Color}};
use pavex_cli_client::commands::generate::GenerateError;

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let outcome = Client::new()
        .color(Color::Always)
        .pavex_cli_path(r#"{}"#.into())
        .generate(blueprint(), "{}".into())
        .diagnostics_path("diagnostics.dot".into())
        .execute();
    match outcome {{
        Ok(_) => {{}},
        Err(GenerateError::NonZeroExitCode(_)) => {{ std::process::exit(1); }}
        Err(e) => {{
            eprintln!("Failed to invoke `pavex generate`.\n{{:?}}", e);
            std::process::exit(1);
        }}
    }}
    Ok(())
}}
"##,
            self.name_hash,
            pavex_cli.to_str().unwrap(),
            self.runtime_directory.join("generated_app").display(),
        );
        persist_if_changed(&source_directory.join("main.rs"), main_rs.as_bytes())?;
        Ok(())
    }

    pub fn should_run_tests(&self) -> bool {
        self.has_tests
    }
}

#[derive(Debug, PartialEq, Eq)]
enum CodegenTestOutcome {
    Success,
    Failure { msg: String },
}

impl CodegenTestOutcome {
    fn into_trial(
        self,
        test_name: &str,
        test_config: &TestConfig,
        codegen_output: Option<&CommandOutput>,
    ) -> Trial {
        let codegen_test_name = format!("{test_name}::codegen");
        match self {
            CodegenTestOutcome::Success => Trial::test(codegen_test_name, || Ok(())),
            CodegenTestOutcome::Failure { msg } => {
                let msg = if let Some(codegen_output) = codegen_output {
                    enrich_codegen_failure_message(codegen_output, test_config, &msg)
                } else {
                    msg
                };
                let msg = enrich_failure_message(test_config, msg);
                Trial::test(codegen_test_name, move || Err(Failed::from(msg)))
            }
        }
    }
}

fn code_generation_test(
    runtime_directory: &Path,
    test: &TestData,
    pavexc_cli: &Path,
) -> (Option<CommandOutput>, CodegenTestOutcome) {
    let binary_name = format!("app_{}", test.name_hash);
    let binary = runtime_directory
        .join("target")
        .join("debug")
        .join(&binary_name);
    let output = match std::process::Command::new(binary)
        .env("PAVEX_PAVEXC", pavexc_cli)
        .current_dir(&test.runtime_directory)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            let msg = format!("Failed to invoke the code generator.\n{:?}", e);
            return (None, CodegenTestOutcome::Failure { msg });
        }
    };
    let codegen_output = match CommandOutput::try_from(&output) {
        Ok(o) => o,
        Err(e) => {
            let msg = format!("Failed to convert the code generator output.\n{:?}", e);
            return (None, CodegenTestOutcome::Failure { msg });
        }
    };

    let expectations_directory = test.expectations_directory();
    let outcome = if !output.status.success() {
        match test.configuration.expectations.codegen {
            ExpectedOutcome::Pass => CodegenTestOutcome::Failure {
                msg: "We failed to generate the application code.".into(),
            },
            ExpectedOutcome::Fail => {
                let stderr_snapshot = SnapshotTest::new(
                    expectations_directory.join("stderr.txt"),
                    test.blueprint_crate_name(),
                );
                if stderr_snapshot.verify(&codegen_output.stderr).is_err() {
                    CodegenTestOutcome::Failure { msg: "The failure message returned by code generation doesn't match what we expected".into() }
                } else {
                    CodegenTestOutcome::Success
                }
            }
        }
    } else if ExpectedOutcome::Fail == test.configuration.expectations.codegen {
        CodegenTestOutcome::Failure {
            msg: "We expected code generation to fail, but it succeeded!".into(),
        }
    } else {
        CodegenTestOutcome::Success
    };
    (Some(codegen_output), outcome)
}

fn code_generation_lints_test(
    data: &TestData,
    test_name: &str,
    codegen_output: &CommandOutput,
) -> Option<Trial> {
    let ExpectedOutcome::Fail = data.configuration.expectations.lints else {
        return None;
    };

    let stderr_snapshot = SnapshotTest::new(
        data.expectations_directory().join("stderr.txt"),
        data.blueprint_crate_name(),
    );
    let lints_test_name = format!("{test_name}::codegen_lints");
    let trial = if stderr_snapshot.verify(&codegen_output.stderr).is_err() {
        let msg = enrich_codegen_failure_message(
            &codegen_output,
            &data.configuration,
            "The warnings returned by code generation don't match what we expected",
        );
        Trial::test(lints_test_name, move || Err(Failed::from(msg)))
    } else {
        Trial::test(lints_test_name, || Ok(()))
    };
    Some(trial)
}

fn enrich_codegen_failure_message(
    codegen_output: &CommandOutput,
    configuration: &TestConfig,
    msg: &str,
) -> String {
    let msg = format!(
        "{msg}\n\nCODEGEN:\n\t--- STDOUT:\n{}\n\t--- STDERR:\n{}",
        codegen_output.stdout, codegen_output.stderr
    );
    enrich_failure_message(configuration, msg)
}

fn code_generation_diagnostics_test(test_name: &str, test: &TestData) -> Trial {
    let test_name = format!("{test_name}::codegen_diagnostics");
    let expectations_directory = test.expectations_directory();
    let diagnostics_snapshot = SnapshotTest::new(
        expectations_directory.join("diagnostics.dot"),
        test.blueprint_crate_name(),
    );
    let actual_diagnostics =
        match fs_err::read_to_string(test.runtime_directory.join("diagnostics.dot")) {
            Ok(d) => d,
            Err(e) => {
                let msg = format!(
                "Code generation didn't produce a diagnostic file in the expected location.\n{:?}",
                e
            );
                return Trial::test(test_name, move || Err(Failed::from(msg)));
            }
        };
    if diagnostics_snapshot.verify(&actual_diagnostics).is_err() {
        let msg =
            "The diagnostics returned by code generation don't match what we expected.".to_string();
        Trial::test(test_name, move || Err(Failed::from(msg)))
    } else {
        Trial::test(test_name, || Ok(()))
    }
}

fn application_code_test(test_name: &str, test: &TestData) -> Trial {
    let test_name = format!("{test_name}::app_code");
    let expectations_directory = test.expectations_directory();
    let app_code_snapshot = SnapshotTest::new(
        expectations_directory.join("app.rs"),
        test.blueprint_crate_name(),
    );
    let generated_code_path = test.generated_app_directory().join("src").join("lib.rs");
    let actual_app_code = match fs_err::read_to_string(generated_code_path) {
        Ok(d) => d,
        Err(e) => {
            let msg = format!(
                "Code generation didn't produce a `lib.rs` file in the expected location.\n{:?}",
                e
            );
            return Trial::test(test_name, move || Err(Failed::from(msg)));
        }
    };
    if app_code_snapshot.verify(&actual_app_code).is_err() {
        let msg = "The generated application code doesn't match what we expected.".to_string();
        Trial::test(test_name, move || Err(Failed::from(msg)))
    } else {
        Trial::test(test_name, || Ok(()))
    }
}

fn application_code_compilation_test(test: &TestData) -> Result<(), anyhow::Error> {
    // Check that the generated code compiles
    let output = std::process::Command::new("cargo")
        // .env("RUSTFLAGS", "-Awarnings")
        .arg("check")
        .arg("--jobs")
        .arg("1")
        .arg("-p")
        .arg(format!("application_{}", test.name_hash))
        .arg("--quiet")
        .current_dir(&test.runtime_directory)
        .output()
        .context("Failed to invoke `cargo check` on the generated application code")?;
    let compilation_output: Result<CommandOutput, _> = (&output).try_into();
    let compilation_output =
        compilation_output.context("The output of `cargo check` contains non-UTF8 characters")?;
    if !output.status.success() {
        anyhow::bail!(
            "The generated application code doesn't compile.\n\nCARGO CHECK:\n\t--- STDOUT:\n{}\n\t--- STDERR:\n{}",
                compilation_output.stdout, compilation_output.stderr
        )
    } else {
        Ok(())
    }
}

fn application_integration_test(test: &TestData) -> Result<(), anyhow::Error> {
    let output = std::process::Command::new("cargo")
        // .env("RUSTFLAGS", "-Awarnings")
        .arg("t")
        .arg("--jobs")
        .arg("1")
        .arg("-p")
        .arg(format!("integration_{}", test.name_hash))
        .current_dir(&test.runtime_directory)
        .output()
        .context("Failed to invoke `cargo test` on the app integration tests")?;
    let test_output: CommandOutput = (&output)
        .try_into()
        .context("The output of `cargo test` contains non-UTF8 characters")?;
    if !output.status.success() {
        anyhow::bail!(
                "Integration tests didn't succeed.\n\nCARGO TEST:\n\t--- STDOUT:\n{}\n\t--- STDERR:\n{}",
                test_output.stdout, test_output.stderr
            )
    } else {
        Ok(())
    }
}

/// A refined `std::process::Output` that assumes that both stderr and stdout are valid UTF8.
struct CommandOutput {
    stdout: String,
    stderr: String,
}

impl TryFrom<&Output> for CommandOutput {
    type Error = anyhow::Error;

    fn try_from(o: &Output) -> Result<Self, Self::Error> {
        let stdout = std::str::from_utf8(&o.stdout)
            .context("The application printed invalid UTF8 data to stdout")?;
        let stderr = std::str::from_utf8(&o.stderr)
            .context("The application printed invalid UTF8 data to stderr")?;
        Ok(Self {
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
        })
    }
}

fn enrich_failure_message(config: &TestConfig, error: impl AsRef<str>) -> String {
    let description = style(textwrap::indent(&config.description, "    ")).cyan();
    let error = style(textwrap::indent(error.as_ref(), "    ")).red();
    format!(
        "{}\n{description}.\n{}\n{error}",
        style("What is the test about:").cyan().dim().bold(),
        style("What went wrong:").red().bold(),
    )
}
