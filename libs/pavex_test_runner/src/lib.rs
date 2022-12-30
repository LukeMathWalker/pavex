use std::collections::HashMap;
use std::fmt::Write;
use std::path::PathBuf;
use std::process::Output;

use anyhow::Context;
use console::style;
use libtest_mimic::{Conclusion, Failed};
use toml::toml;

pub use snapshot::print_changeset;

use crate::snapshot::SnapshotTest;

mod snapshot;

/// Create a test case for each folder in `definition_directory`.
///
/// Each test will get a separate runtime environment - a sub-folder of `runtime_directory`. The
/// same sub-folder is reused across multiple test runs to benefit from cargo's incremental compilation.
///
/// Custom configuration can be specified on a per-test basis by including a `test_config.toml` file
/// in the test folder. The available test options are detailed in [`TestConfig`].
///
/// # cargo-nextest
///
/// Our custom test runner is built on top of `libtest_mimic`, which gives us
/// [compatibility out-of-the-box](https://nexte.st/book/custom-test-harnesses.html) with `cargo-nextest`.
pub fn run_tests(
    definition_directory: PathBuf,
    runtime_directory: PathBuf,
) -> Result<Conclusion, anyhow::Error> {
    let arguments = libtest_mimic::Arguments::from_args();

    let entries = fs_err::read_dir(&definition_directory)?;
    let mut tests = Vec::new();
    for entry in entries {
        let entry = entry?;
        let filename = entry.file_name();
        let name = filename
            .to_str()
            .expect("The name of test folders must be valid unicode.")
            .to_owned();
        let test_data = TestData {
            definition_directory: entry.path(),
            runtime_directory: runtime_directory.join("tests").join(filename),
        };
        let test_configuration = test_data
            .load_configuration()
            .expect("Failed to load test configuration");
        let is_ignored = test_configuration.ignore;
        let test = libtest_mimic::Trial::test(name.clone(), move || {
            run_test(test_data, test_configuration)
        })
        .with_ignored_flag(is_ignored);
        tests.push(test);
    }
    Ok(libtest_mimic::run(&arguments, tests))
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
    /// `pavex` itself.
    #[serde(default)]
    dependencies: toml::value::Table,
    /// Ignore the test if set to `true`.
    #[serde(default)]
    ignore: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
struct EphemeralDependency {
    /// The path to the file that should be used as `lib.rs` in the generated library crate.
    path: PathBuf,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
struct TestExpectations {
    /// By default, we expect code generation (i.e. `app.build()`) to succeed.
    /// If set to `fail`, the test runner will look for a snapshot of the expected failure message
    /// returned by `pavex` to the user.
    #[serde(default = "ExpectedOutcome::pass")]
    codegen: ExpectedOutcome,
}

impl Default for TestExpectations {
    fn default() -> Self {
        Self {
            codegen: ExpectedOutcome::Pass,
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
    definition_directory: PathBuf,
    runtime_directory: PathBuf,
}

impl TestData {
    fn load_configuration(&self) -> Result<TestConfig, anyhow::Error> {
        let test_config =
            fs_err::read_to_string(self.definition_directory.join("test_config.toml")).context(
                "All UI tests must have an associated `test_config.toml` file with, \
                    at the very least, a `description` field explaining what the test is trying \
                    to verify.",
            )?;
        toml::from_str(&test_config).context(
            "Failed to deserialize `test_config.toml`. Check the file against the expected schema!",
        )
    }

    /// Populate the runtime test folder using the directives and the files in the test
    /// definition folder.
    fn seed_test_filesystem(&self, test_config: &TestConfig) -> Result<(), anyhow::Error> {
        let source_directory = self.runtime_directory.join("src");
        fs_err::create_dir_all(&source_directory).context(
            "Failed to create the runtime directory when setting up the test runtime environment",
        )?;
        fs_err::copy(
            self.definition_directory.join("lib.rs"),
            source_directory.join("lib.rs"),
        )?;

        let deps_subdir = self.runtime_directory.join("ephemeral_dependencies");

        for (dependency_name, filepath) in &test_config.ephemeral_dependencies {
            let dep_runtime_directory = deps_subdir.join(dependency_name);
            let dep_source_directory = dep_runtime_directory.join("src");
            fs_err::create_dir_all(&dep_source_directory).context(
                "Failed to create the source directory for an ephemeral dependency when setting up the test runtime environment",
            )?;
            fs_err::copy(
                self.definition_directory.join(&filepath.path),
                dep_source_directory.join("lib.rs"),
            )?;
            let mut cargo_toml = toml! {
                [package]
                name = "dummy"
                version = "0.1.0"
                edition = "2021"
            };
            cargo_toml["package"]["name"] = dependency_name.to_owned().into();
            fs_err::write(
                dep_runtime_directory.join("Cargo.toml"),
                toml::to_string(&cargo_toml)?,
            )?;
        }

        let mut cargo_toml = toml! {
            [workspace]
            members = ["."]

            [package]
            name = "app"
            version = "0.1.0"
            edition = "2021"

            [dependencies]
            pavex_builder = { path = "../../../libs/pavex_builder" }
            pavex_runtime = { path = "../../../libs/pavex_runtime" }
        };
        if !test_config.ephemeral_dependencies.is_empty() {
            cargo_toml["workspace"]["members"] = vec![".", "ephemeral_dependencies/*"].into();
        }
        let deps = cargo_toml
            .get_mut("dependencies")
            .unwrap()
            .as_table_mut()
            .unwrap();
        deps.extend(test_config.dependencies.clone());
        let ephemeral_dependencies = test_config.ephemeral_dependencies.keys().map(|name| {
            let mut value = toml::value::Table::new();
            value.insert(
                "path".into(),
                format!("ephemeral_dependencies/{name}").into(),
            );
            (name.to_owned(), toml::Value::Table(value))
        });
        deps.extend(ephemeral_dependencies);

        fs_err::write(
            self.runtime_directory.join("Cargo.toml"),
            toml::to_string(&cargo_toml)?,
        )?;

        // Use sccache to avoid rebuilding the same dependencies
        // over and over again.
        let cargo_config = toml! {
            [build]
            rustc-wrapper = "sccache"
        };
        let dot_cargo_folder = self.runtime_directory.join(".cargo");
        fs_err::create_dir_all(&dot_cargo_folder)?;
        fs_err::write(
            dot_cargo_folder.join("config.toml"),
            toml::to_string(&cargo_config)?,
        )?;

        let main_rs = r#"use app::blueprint;
use std::str::FromStr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from_str("blueprint.json")?;
    blueprint().persist(&path)?;

    let status = std::process::Command::new("../../../target/debug/pavex_cli")
        .arg("generate")
        .arg("-b")
        .arg(&path)
        .arg("--diagnostics")
        .arg("diagnostics.dot")
        .arg("-o")
        .arg("generated_app")
        .status()?;
       
    if !status.success() {
        std::process::exit(1); 
    }
     
    Ok(())
}"#;
        fs_err::write(source_directory.join("main.rs"), main_rs)?;
        Ok(())
    }
}

fn run_test(test: TestData, config: TestConfig) -> Result<(), Failed> {
    match _run_test(&config, &test) {
        Ok(TestOutcome {
            outcome: Err(mut msg),
            codegen_output,
            compilation_output,
        }) => Err(Failed::from({
            write!(
                &mut msg,
                "\n\nCODEGEN:\n\t--- STDOUT:\n{}\n\t--- STDERR:\n{}",
                codegen_output.stdout, codegen_output.stderr
            )
            .unwrap();
            if let Some(compilation_output) = compilation_output {
                write!(
                    &mut msg,
                    "\n\nCARGO CHECK:\n\t--- STDOUT:\n{}\n\t--- STDERR:\n{}",
                    compilation_output.stdout, compilation_output.stderr
                )
                .unwrap();
            }
            enrich_failure_message(&config, msg)
        })),
        Err(e) => Err(Failed::from(enrich_failure_message(
            &config,
            unexpected_failure_message(&e),
        ))),
        Ok(TestOutcome {
            outcome: Ok(()), ..
        }) => Ok(()),
    }
}

fn _run_test(test_config: &TestConfig, test: &TestData) -> Result<TestOutcome, anyhow::Error> {
    test.seed_test_filesystem(test_config)
        .context("Failed to seed the filesystem for the test runtime folder")?;

    // Generate the application code
    let output = std::process::Command::new("cargo")
        .env("RUSTFLAGS", "-Awarnings")
        .arg("run")
        .arg("--quiet")
        .current_dir(&test.runtime_directory)
        .output()
        .unwrap();
    let codegen_output: CommandOutput = (&output).try_into()?;

    let expectations_directory = test.definition_directory.join("expectations");

    if !output.status.success() {
        return match test_config.expectations.codegen {
            ExpectedOutcome::Pass => Ok(TestOutcome {
                outcome: Err("We failed to generate the application code.".to_string()),
                codegen_output,
                compilation_output: None,
            }),
            ExpectedOutcome::Fail => {
                let stderr_snapshot = SnapshotTest::new(expectations_directory.join("stderr.txt"));
                if stderr_snapshot.verify(&codegen_output.stderr).is_err() {
                    return Ok(TestOutcome {
                        outcome: Err("The failure message returned by code generation does not match what we expected".into()),
                        codegen_output,
                        compilation_output: None,
                    });
                }
                Ok(TestOutcome {
                    outcome: Ok(()),
                    codegen_output,
                    compilation_output: None,
                })
            }
        };
    } else if ExpectedOutcome::Fail == test_config.expectations.codegen {
        return Ok(TestOutcome {
            outcome: Err("We expected code generation to fail, but it succeeded!".into()),
            codegen_output,
            compilation_output: None,
        });
    };

    let diagnostics_snapshot = SnapshotTest::new(expectations_directory.join("diagnostics.dot"));
    let actual_diagnostics =
        fs_err::read_to_string(test.runtime_directory.join("diagnostics.dot"))?;
    if diagnostics_snapshot.verify(&actual_diagnostics).is_err() {
        return Ok(TestOutcome {
            outcome: Err(
                "Diagnostics for the generated application do not match what we expected".into(),
            ),
            codegen_output,
            compilation_output: None,
        });
    }

    let app_code_snapshot = SnapshotTest::new(expectations_directory.join("app.rs"));
    let actual_app_code = fs_err::read_to_string(
        test.runtime_directory
            .join("generated_app")
            .join("src")
            .join("lib.rs"),
    )
    .unwrap();
    if app_code_snapshot.verify(&actual_app_code).is_err() {
        return Ok(TestOutcome {
            outcome: Err("The generated application code does not match what we expected".into()),
            codegen_output,
            compilation_output: None,
        });
    }

    // Check that the generated code compiles
    let output = std::process::Command::new("cargo")
        .env("RUSTFLAGS", "-Awarnings")
        .arg("check")
        .arg("--workspace")
        .arg("--quiet")
        .current_dir(&test.runtime_directory)
        .output()
        .unwrap();
    let compilation_output: CommandOutput = (&output).try_into()?;
    if !output.status.success() {
        return Ok(TestOutcome {
            outcome: Err("The generated application code does not compile.".into()),
            codegen_output,
            compilation_output: Some(compilation_output),
        });
    }

    Ok(TestOutcome {
        outcome: Ok(()),
        codegen_output,
        compilation_output: Some(compilation_output),
    })
}

struct TestOutcome {
    outcome: Result<(), String>,
    codegen_output: CommandOutput,
    compilation_output: Option<CommandOutput>,
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

fn unexpected_failure_message(e: &anyhow::Error) -> String {
    format!(
        "An unexpected error was encountered when running a test.\n\n{}\n---\n{:?}",
        &e, &e
    )
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
