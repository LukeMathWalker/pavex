use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::io::BufWriter;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output};

use ahash::HashSet;
use anyhow::Context;
use cargo_metadata::diagnostic::DiagnosticLevel;
use console::style;
use guppy::graph::PackageGraph;
use itertools::Itertools;
use libtest_mimic::{Arguments, Conclusion, Failed, Trial};
use pavexc::DEFAULT_DOCS_TOOLCHAIN;
use pavexc::rustdoc::CrateCollection;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use sha2::Digest;
use toml::toml;
use walkdir::WalkDir;

use persist_if_changed::persist_if_changed;
pub use snapshot::print_changeset;

use crate::snapshot::SnapshotTest;

mod snapshot;

/// Return an iterator over the directories containing a UI test.
pub fn get_ui_test_directories(test_folder: &Path) -> impl Iterator<Item = PathBuf> + use<> {
    WalkDir::new(test_folder)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name() == "test_config.toml")
        .map(|entry| entry.path().parent().unwrap().to_path_buf())
}

pub fn get_test_name(tests_parent_folder: &Path, test_folder: &Path) -> String {
    let relative_path = test_folder.strip_prefix(tests_parent_folder).unwrap();
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
    tests_directory: PathBuf,
) -> Result<Conclusion, anyhow::Error> {
    let arguments = libtest_mimic::Arguments::from_args();

    let _ = increase_open_file_descriptor_limit();

    let mut test_name2test_data = BTreeMap::new();
    for entry in get_ui_test_directories(&tests_directory) {
        let test_name = get_test_name(&tests_directory, &entry);
        let test_data = TestData::new(&test_name, entry.clone())?;
        test_name2test_data.insert(test_name, test_data);
    }

    create_tests_dir(&tests_directory, &test_name2test_data)?;

    let test_name2test_data = test_name2test_data
        .into_iter()
        .filter(|(test_name, test_data)| {
            // Skip tests that are filtered out (e.g. by name or by tag)
            // We only do this *after* we've created the test directories, so that we don't
            // change the test directory structure based on the filters, which would cause
            // cache invalidation issues for `cargo`.
            !is_filtered_out(&arguments, test_name) && !test_data.configuration.ignore
        })
        .collect::<BTreeMap<_, _>>();

    let mut trials = Vec::with_capacity(test_name2test_data.len() * 4);
    if !arguments.list {
        warm_up_target_dir(&tests_directory, &test_name2test_data)?;

        let metadata = guppy::MetadataCommand::new()
            .current_dir(&tests_directory)
            .exec()
            .context("Failed to invoke `cargo metadata`")?;

        let metadata_path = tests_directory.join("metadata.json");

        {
            use std::io::Write as _;

            let mut file = BufWriter::new(fs_err::File::create(&metadata_path)?);
            metadata
                .serialize(&mut file)
                .context("Failed to serialize Cargo's metadata to disk")?;
            file.flush()
                .context("Failed to serialize Cargo's metadata to disk")?;
        }

        let package_graph = metadata
            .build_graph()
            .context("Failed to build package graph")?;

        // We warm up Pavex's `rustdoc` cache to avoid cross-test contention.
        // In case of a cache miss, a UI test will try to run `rustdoc` and
        // acquire the global target directory lock. If multiple tests end up
        // doing this at the same time, their execution will be serialized,
        // which would have a major impact on the overall test suite runtime.
        warm_up_rustdoc_cache(&package_graph, &test_name2test_data)?;

        // First battery of UI tests.
        // For each UI test, we run code generation and then generate a different
        // "test case" for each assertion that's relevant to the code generation process.
        // If all assertions pass, we return a `bool` set to `true` which indicates
        // that we want to run the second battery of assertions on that UI test.
        let n_codegen_tests = test_name2test_data.len();
        let timer = std::time::Instant::now();
        println!("Performing code generation for {n_codegen_tests} tests");
        let intermediate: BTreeMap<String, (Vec<Trial>, bool)> = test_name2test_data
            // This defaults to the number of logical cores on the machine.
            // TODO: honor the `--jobs` flag from `cargo test`.
            .par_iter()
            .map(|(name, data)| {
                let mut trials = Vec::new();
                let (codegen_output, outcome) = code_generation_test(
                    &tests_directory,
                    data,
                    &pavexc_cli,
                    &pavex_cli,
                    &metadata_path,
                );
                let is_success = outcome == CodegenTestOutcome::Success;
                let trial = outcome.into_trial(name, &data.configuration, codegen_output.as_ref());
                trials.push(trial);

                // If the code generation test failed, we skip the follow-up tests.
                if !is_success {
                    return (name.to_owned(), (trials, false));
                }

                if let Some(codegen_output) = codegen_output {
                    if let Some(trial) = code_generation_lints_test(data, name, &codegen_output) {
                        trials.push(trial);
                    }
                };

                if data.configuration.expectations.codegen == ExpectedOutcome::Fail {
                    return (name.to_owned(), (trials, false));
                }

                let trial = code_generation_diagnostics_test(name, data);
                trials.push(trial);

                let trial = application_code_test(name, data);
                trials.push(trial);
                (name.to_owned(), (trials, true))
            })
            .collect();
        println!(
            "Performed code generation for {n_codegen_tests} tests in {} seconds",
            timer.elapsed().as_secs()
        );

        let test_name2test_data: BTreeMap<_, _> = test_name2test_data
            .into_iter()
            .filter(|(k, _)| intermediate[k].1)
            .collect();

        // Save the results of the first battery of tests.
        intermediate.into_values().for_each(|(partial, _)| {
            trials.extend(partial);
        });

        let (cases, test_name2test_data) =
            compile_generated_apps(&tests_directory, test_name2test_data)?;

        trials.extend(cases);

        let test_name2test_data: BTreeMap<_, _> = test_name2test_data
            .into_iter()
            .filter(|(_, data)| data.should_run_tests())
            .collect();
        build_integration_tests(&tests_directory, &test_name2test_data);
        let n_integration_cases = test_name2test_data.len();
        println!("Running integration tests for {n_integration_cases} test cases");
        let timer = std::time::Instant::now();
        for (name, data) in test_name2test_data {
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
        println!(
            "Ran integration tests for {n_integration_cases} test cases in {} seconds",
            timer.elapsed().as_secs()
        );
    }
    Ok(libtest_mimic::run(&arguments, trials))
}

/// Try to increase the open file descriptor limit.
/// The default limit is usually too low for integration tests,
/// causing tests to fail with `Too many open files` errors.
fn increase_open_file_descriptor_limit() -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::cmp::{max, min};

        let (soft, hard) = rlimit::Resource::NOFILE.get()?;
        let target = max(soft, min(10_000, hard));
        rlimit::Resource::NOFILE.set(target, hard)?;
    }
    Ok(())
}

fn compile_generated_apps(
    runtime_directory: &Path,
    test_name2test_data: BTreeMap<String, TestData>,
) -> Result<(Vec<Trial>, BTreeMap<String, TestData>), anyhow::Error> {
    let generated_crate_names = test_name2test_data
        .values()
        .map(|data| data.generated_crate_name())
        .collect::<BTreeSet<_>>();
    let timer = std::time::Instant::now();
    println!("Compiling {} generated crates", generated_crate_names.len());
    let mut cmd = Command::new("cargo");
    cmd.arg("check").arg("--message-format").arg("json");
    for name in &generated_crate_names {
        cmd.arg("-p").arg(name);
    }
    let output = cmd
        .current_dir(runtime_directory)
        .output()
        .context("Failed to invoke `cargo build` on the generated crates")?;
    let build_output = CommandOutput::try_from(&output).context("Failed to parse build output")?;
    let mut crate_names2error = BTreeMap::<_, String>::new();
    for line in build_output
        .stderr
        .lines()
        .chain(build_output.stdout.lines())
    {
        let Ok(cargo_metadata::Message::CompilerMessage(msg)) =
            serde_json::from_str::<cargo_metadata::Message>(line)
        else {
            // Not all output lines are JSON, so we ignore the ones that aren't.
            // We also ignore other types of messages (e.g. build scripts notifications
            // and artifact information).
            continue;
        };
        // We are only looking at errors here.
        if !(msg.message.level == DiagnosticLevel::Error
            || msg.message.level == DiagnosticLevel::FailureNote)
        {
            continue;
        }

        let package_name_and_version = msg
            .package_id
            .repr
            .split_once('#')
            .expect("Missing package name")
            .1;
        let package_name = package_name_and_version
            .split_once('@')
            .expect("Missing version")
            .0;
        assert!(
            generated_crate_names.contains(package_name),
            "Error compiling a crate that's not one of ours, {}",
            msg.package_id.repr
        );
        let errors = crate_names2error
            .entry(package_name.to_owned())
            .or_default();
        writeln!(
            errors,
            "{}",
            msg.message.rendered.unwrap_or(msg.message.message)
        )
        .unwrap();
    }
    let mut trials = Vec::new();
    let mut further = BTreeMap::new();
    for (test_name, data) in test_name2test_data {
        let error = crate_names2error.get(&data.generated_crate_name()).cloned();
        let case_name = format!("{test_name}::app_code_compiles");
        let trial = if let Some(error) = error {
            Trial::test(case_name, move || Err(Failed::from(error)))
        } else {
            further.insert(test_name, data);
            Trial::test(case_name, || Ok(()))
        };
        trials.push(trial);
    }
    if !output.status.success() && crate_names2error.is_empty() {
        panic!(
            "Something went wrong when compiling the generated crates, but we failed to capture what or where."
        )
    }
    println!(
        "Compiled {} generated crates in {} seconds",
        generated_crate_names.len(),
        timer.elapsed().as_secs(),
    );
    Ok((trials, further))
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
fn warm_up_target_dir(
    runtime_directory: &Path,
    test_name2test_data: &BTreeMap<String, TestData>,
) -> Result<(), anyhow::Error> {
    let timer = std::time::Instant::now();
    println!("Warming up the target directory");
    let mut cmd = Command::new("cargo");
    cmd.arg("build");
    for data in test_name2test_data.values() {
        cmd.arg("-p").arg(data.blueprint_crate_name());
    }
    cmd.arg("--all-targets")
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
    package_graph: &PackageGraph,
    test_name2test_data: &BTreeMap<String, TestData>,
) -> Result<(), anyhow::Error> {
    let timer = std::time::Instant::now();
    // We want to ensure that all invocations of `pavexc generate` hit the cache
    // thus avoiding the need to invoke `rustdoc` and acquire a contentious
    // lock over the target directory.
    println!("Pre-computing JSON documentation for relevant crates");
    let crate_collection = CrateCollection::new(
        DEFAULT_DOCS_TOOLCHAIN.to_owned(),
        package_graph.clone(),
        package_graph.workspace().root().to_string(),
        true,
    )?;
    let app_names = test_name2test_data
        .values()
        .map(|data| data.blueprint_crate_name())
        .collect::<HashSet<_>>();
    let mut crates: HashSet<_> = crate_collection
        .package_graph()
        .workspace()
        .iter()
        .filter(|p| {
            !(p.name().starts_with("application_")
                || p.name().starts_with("integration_")
                || p.name() == "workspace_hack"
                || (p.name().starts_with("app_") && !app_names.contains(p.name())))
        })
        .map(|p| p.name().to_owned())
        .collect();
    crates.remove("workspace_hack");
    // Toolchain docs
    crates.insert("core".into());
    crates.insert("alloc".into());
    crates.insert("std".into());
    // Packages that depend on `pavex` and will be used in UI tests
    crates.insert("pavex".into());
    crates.insert("pavex_cli_client".into());
    crates.insert("pavex_macros".into());
    // Hand-picked crates that we know we're going to build docs for.
    crates.insert("tracing".into());
    crates.insert("equivalent".into());
    crates.insert("ppv-lite86".into());
    crates.insert("hashbrown".into());
    crates.insert("typenum".into());
    crates.insert("http".into());
    crates.insert("anyhow".into());
    crates.insert("pear".into());
    crates.insert("yansi".into());
    crates.insert("serde".into());
    crates.insert("zerocopy".into());
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
) -> Result<(), anyhow::Error> {
    let timer = std::time::Instant::now();
    println!("Seeding the filesystem");
    fs_err::create_dir_all(runtime_directory)
        .context("Failed to create runtime directory for UI tests")?;

    // Create a `Cargo.toml` to define a workspace,
    // where each UI test is a workspace member
    let cargo_toml_path = runtime_directory.join("Cargo.toml");
    let mut cargo_toml = r##"# Generated by `pavex_test_runner`.
# Do NOT modify it manually.
[profile.dev]
# Minimise the amount of disk space used by the build artifacts.
debug = "none""##
        .to_string();
    writeln!(&mut cargo_toml, "\n[workspace]\nmembers = [").unwrap();
    for test_data in test_name2test_data.values() {
        for member in test_data.workspace_members() {
            let relative_path = member.strip_prefix(runtime_directory).unwrap();
            // We use the Unix path separator, since that's what `cargo` expects
            // in `Cargo.toml` files.
            let p = relative_path
                .components()
                .map(|c| match c {
                    std::path::Component::Normal(s) => s.to_string_lossy(),
                    _ => unreachable!(),
                })
                .join("/");
            writeln!(cargo_toml, "  \"{p}\",").unwrap();
        }
    }
    writeln!(&mut cargo_toml, "]").unwrap();
    writeln!(&mut cargo_toml, "resolver = \"3\"").unwrap();
    writeln!(&mut cargo_toml, "[workspace.package]\nedition = \"2024\"")?;
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
    writeln!(&mut cargo_toml, "reqwest = \"0.12\"").unwrap();
    writeln!(&mut cargo_toml, "tokio = \"1\"").unwrap();
    writeln!(
        &mut cargo_toml,
        r#"serde = {{ version = "1", features = ["derive"] }}"#
    )
    .unwrap();

    persist_if_changed(&cargo_toml_path, cargo_toml.as_bytes())?;

    // Create a manifest for each UI test
    // Each UI test is composed of multiple crates, therefore we nest
    // everything under a test-specific directory to avoid name
    // clashes and confusion across tests
    for test_data in test_name2test_data.values() {
        test_data.reset_test_filesystem()?;
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
    /// Ignore the test if set to `true`.
    #[serde(default)]
    ignore: bool,
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
    configuration: TestConfig,
    has_tests: bool,
}

impl TestData {
    fn new(test_name: &str, definition_directory: PathBuf) -> Result<Self, anyhow::Error> {
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
        let integration_test_file = definition_directory.join("integration");
        let has_tests = integration_test_file.exists();
        Ok(Self {
            name_hash,
            definition_directory,
            configuration,
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

    fn blueprint_directory(&self) -> &Path {
        &self.definition_directory
    }

    fn expectations_directory(&self) -> PathBuf {
        self.definition_directory.join("expectations")
    }

    fn generated_app_directory(&self) -> PathBuf {
        self.definition_directory.join("generated_app")
    }

    fn blueprint_crate_name(&self) -> String {
        format!("app_{}", self.name_hash)
    }

    fn generated_crate_name(&self) -> String {
        format!("application_{}", self.name_hash)
    }

    fn integration_test_directory(&self) -> Option<PathBuf> {
        self.has_tests
            .then(|| self.definition_directory.join("integration"))
    }

    fn reset_test_filesystem(&self) -> Result<(), anyhow::Error> {
        // Empty application crate, ahead of code generation.
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
                generator_name = "dummy"
            };
            cargo_toml["package"]["name"] = format!("application_{}", self.name_hash).into();
            cargo_toml["package"]["metadata"]["px"]["generate"]["generator_name"] =
                format!("app_{}", self.name_hash).into();
            persist_if_changed(
                &application_dir.join("Cargo.toml"),
                toml::to_string(&cargo_toml)?.as_bytes(),
            )?;
        }

        // We manage here the code generator binary for each UI test
        // to avoid code drifting
        {
            let main_rs = format!(
                r##"//! This code is generated by `pavex_test_runner`,
//! Do NOT modify it manually.
use app_{}::blueprint;
use pavex_cli_client::{{Client, config::Color}};
use pavex_cli_client::commands::generate::GenerateError;

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let ui_test_dir: std::path::PathBuf = std::env::var("UI_TEST_DIR").unwrap().into();
    let outcome = Client::new()
        .color(Color::Always)
        .pavex_cli_path(std::env::var("PAVEX_TEST_CLI_PATH").unwrap().into())
        .generate(blueprint(), ui_test_dir.join("generated_app"))
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
                self.name_hash
            );
            persist_if_changed(
                &self.blueprint_directory().join("src").join("main.rs"),
                main_rs.as_bytes(),
            )?;
        }
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
    pavex_cli: &Path,
    metadata: &Path,
) -> (Option<CommandOutput>, CodegenTestOutcome) {
    let binary_name = format!("app_{}", test.name_hash);
    let binary = runtime_directory
        .join("target")
        .join("debug")
        .join(&binary_name);
    let output = match std::process::Command::new(binary)
        .env("PAVEX_TEST_CLI_PATH", pavex_cli)
        .env("UI_TEST_DIR", &test.definition_directory)
        .env("PAVEX_PAVEXC", pavexc_cli)
        .env("PAVEXC_CACHE_WORKSPACE_PACKAGES", "true")
        .env("PAVEXC_PRECOMPUTED_METADATA", metadata)
        .current_dir(&test.definition_directory)
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

    if std::env::var("PAVEX_LOG").as_deref() == Ok("true") {
        eprintln!(
            "Code generation stderr:\n{}Code generation stdout:\n{}",
            textwrap::indent(&codegen_output.stderr, "    "),
            textwrap::indent(&codegen_output.stdout, "    "),
        )
    }

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
            codegen_output,
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
    let actual_diagnostics = match fs_err::read_to_string(
        test.definition_directory.join("diagnostics.dot"),
    ) {
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

fn build_integration_tests(test_dir: &Path, test_name2test_data: &BTreeMap<String, TestData>) {
    let n_integration_tests = test_name2test_data.len();
    if n_integration_tests == 0 {
        return;
    }

    let timer = std::time::Instant::now();
    println!("Building {n_integration_tests} integration tests, without running them");
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("test").arg("--no-run");
    for test in test_name2test_data.values() {
        cmd.arg("-p").arg(format!("integration_{}", test.name_hash));
    }

    // We don't care if it fails, since we are going to capture failures
    // one test at a time when we run them.
    let _ = cmd
        .current_dir(test_dir)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output();
    println!(
        "Built {n_integration_tests} integration tests in {} seconds",
        timer.elapsed().as_secs()
    );
}

fn application_integration_test(test: &TestData) -> Result<(), anyhow::Error> {
    let output = std::process::Command::new("cargo")
        // .env("RUSTFLAGS", "-Awarnings")
        .arg("t")
        .arg("-p")
        .arg(format!("integration_{}", test.name_hash))
        .current_dir(&test.definition_directory)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()
        .context("Failed to invoke `cargo test` on the app integration tests")?;
    let test_output: CommandOutput = (&output)
        .try_into()
        .context("The output of `cargo test` contains non-UTF8 characters")?;
    if !output.status.success() {
        anyhow::bail!(
            "Integration tests didn't succeed.\n\nCARGO TEST:\n\t--- STDOUT:\n{}\n\t--- STDERR:\n{}",
            test_output.stdout,
            test_output.stderr
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
