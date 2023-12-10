use std::collections::HashMap;
use std::io::Write;
use std::time::Duration;

use anyhow::Context;
use console::style;
use run_script::types::ScriptOptions;
use similar::{Algorithm, ChangeTag, TextDiff};

#[derive(Debug, serde::Deserialize)]
struct TutorialManifest {
    bootstrap: String,
    starter_project_folder: String,
    steps: Vec<Step>,
}

#[derive(Debug, serde::Deserialize)]
struct Step {
    patch: String,
    #[serde(default)]
    commands: Vec<StepCommand>,
}

#[derive(Debug, serde::Deserialize)]
struct StepCommand {
    command: String,
    expected_outcome: StepCommandOutcome,
    expected_output_at: String,
}

#[derive(Debug, serde::Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
enum StepCommandOutcome {
    Success,
    Failure,
}

fn main() -> Result<(), anyhow::Error> {
    // Check if we have a `--verify` flag
    let verify = {
        let mut args = std::env::args();
        // Skip the first argument, which is the path to the executable
        let _ = args.next();
        args.next().as_deref() == Some("--verify")
    };

    let tutorial_manifest = fs_err::read_to_string("tutorial.yml")
        .context("Failed to open the tutorial manifest file. Are you in the right directory?")?;
    let deserializer = serde_yaml::Deserializer::from_str(&tutorial_manifest);
    let tutorial_manifest: TutorialManifest = serde_path_to_error::deserialize(deserializer)
        .context("Failed to parse the tutorial manifest file")?;

    clean_up();

    // Boostrap the project
    println!("Running bootstrap script");
    let script_outcome =
        run_script(&tutorial_manifest.bootstrap).context("Failed to run the boostrap script")?;
    script_outcome.exit_on_failure("Failed to run the boostrap script");

    // Apply the patches
    let mut previous_dir = tutorial_manifest.starter_project_folder;
    for step in &tutorial_manifest.steps {
        println!("Applying patch: {}", step.patch);
        let next_dir = patch_directory_name(&step.patch);
        let script_outcome = run_script(&format!(
            r#"cp -r {previous_dir} {next_dir}
            cd {next_dir} && patch -p1 < ../{} && git add . && git commit -am "{}""#,
            step.patch, step.patch
        ))
        .context("Failed to apply patch")?;
        script_outcome.exit_on_failure("Failed to apply patch");
        previous_dir = next_dir.to_string();
    }

    let mut errors = vec![];
    for step in &tutorial_manifest.steps {
        for command in &step.commands {
            println!(
                "Running command for patch `{}`: {}",
                step.patch, command.command
            );
            let patch_dir = patch_directory_name(&step.patch);

            assert!(
                command.expected_output_at.ends_with(".snap"),
                "All expected output file must use the `.snap` file extension"
            );

            let script_outcome = run_script(&format!(r#"cd {patch_dir} && {}"#, command.command))?;

            if command.expected_outcome == StepCommandOutcome::Success {
                script_outcome.exit_on_failure("Failed to run command which should have succeeded");
            } else if command.expected_outcome == StepCommandOutcome::Failure {
                script_outcome.exit_on_success("Command succeeded when it should have failed");
            }

            let output = match command.expected_outcome {
                StepCommandOutcome::Success => script_outcome.output,
                StepCommandOutcome::Failure => script_outcome.error,
            };
            if verify {
                let expected_output = fs_err::read_to_string(&command.expected_output_at)
                    .context("Failed to read file")?;
                if expected_output != output {
                    let mut err_msg = format!(
                        "Expected output did not match actual output for patch {} (command: `{}`).\n",
                        step.patch,
                        command.command,
                    );
                    print_changeset(&expected_output, &output, &mut err_msg)?;
                    errors.push(err_msg);
                }
            } else {
                let mut options = fs_err::OpenOptions::new();
                options.write(true).create(true).truncate(true);
                let mut file = options
                    .open(&command.expected_output_at)
                    .context("Failed to open/create expectation file")?;
                file.write_all(output.as_bytes())
                    .expect("Failed to write to expectation file");
            }
        }
    }

    if !errors.is_empty() {
        eprintln!("One or more snapshots didn't match the expected value.");
        for error in errors {
            eprintln!("{}", error);
        }
        std::process::exit(1);
    }

    Ok(())
}

fn patch_directory_name(patch_file: &str) -> &str {
    patch_file
        .strip_suffix(".patch")
        .expect("Patch file didn't use the .patch extension")
}

/// Remove all files from the current directory, recursively, with the exception of
/// top-level *.patch files and the tutorial manifest file.
fn clean_up() {
    fs_err::read_dir(std::env::current_dir().expect("Failed to get the current directory"))
        .expect("Failed to read the current directory")
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let path = entry.path();
            let file_name = path.file_name().unwrap().to_str().unwrap();
            !file_name.ends_with(".patch")
                && !file_name.ends_with(".snap")
                && file_name != "tutorial.yml"
                && file_name != ".gitignore"
        })
        .for_each(|entry| {
            let file_type = entry.file_type().expect("Failed to get file type");
            if file_type.is_dir() {
                fs_err::remove_dir_all(entry.path()).expect("Failed to remove a directory")
            } else {
                fs_err::remove_file(entry.path()).expect("Failed to remove a file")
            }
        });
}

fn run_script(script: &str) -> Result<ScriptOutcome, anyhow::Error> {
    let mut options = ScriptOptions::new();
    let env_vars = HashMap::from([("PAVEX_TTY_WIDTH".to_string(), "100".to_string())]);
    options.env_vars = Some(env_vars);

    run_script::run(script, &Default::default(), &options)
        .map(|(code, output, error)| ScriptOutcome {
            code,
            output,
            error,
        })
        .context("Failed to run script")
}

struct ScriptOutcome {
    code: i32,
    output: String,
    error: String,
}

impl ScriptOutcome {
    fn exit_on_failure(&self, error_msg: &str) {
        if self.code != 0 {
            self.exit(error_msg);
        }
    }

    fn exit_on_success(&self, error_msg: &str) {
        if self.code == 0 {
            self.exit(error_msg);
        }
    }

    fn exit(&self, error_msg: &str) -> ! {
        eprintln!("{error_msg}");
        eprintln!("Exit Code: {}", self.code);
        eprintln!("Output: {}", self.output);
        eprintln!("Error: {}", self.error);
        std::process::exit(1);
    }
}

pub fn print_changeset(
    old: &str,
    new: &str,
    buffer: &mut impl std::fmt::Write,
) -> Result<(), anyhow::Error> {
    let width: usize = 100;
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Patience)
        .timeout(Duration::from_millis(500))
        .diff_lines(old, new);
    writeln!(buffer, "{:─^1$}", "", width)?;

    if !old.is_empty() {
        writeln!(buffer, "{}", style("-old snapshot").red())?;
        writeln!(buffer, "{}", style("+new results").green())?;
    } else {
        writeln!(buffer, "{}", style("+new results").green())?;
    }

    writeln!(buffer, "────────────┬{:─^1$}", "", width.saturating_sub(13))?;
    let mut has_changes = false;
    for (idx, group) in diff.grouped_ops(4).iter().enumerate() {
        if idx > 0 {
            writeln!(buffer, "┈┈┈┈┈┈┈┈┈┈┈┈┼{:┈^1$}", "", width.saturating_sub(13))?;
        }
        for op in group {
            for change in diff.iter_inline_changes(op) {
                match change.tag() {
                    ChangeTag::Insert => {
                        has_changes = true;
                        write!(
                            buffer,
                            "{:>5} {:>5} │{}",
                            "",
                            style(change.new_index().unwrap()).cyan().dim().bold(),
                            style("+").green(),
                        )?;
                        for &(emphasized, change) in change.values() {
                            if emphasized {
                                write!(buffer, "{}", style(change).green().underlined())?;
                            } else {
                                write!(buffer, "{}", style(change).green())?;
                            }
                        }
                    }
                    ChangeTag::Delete => {
                        has_changes = true;
                        write!(
                            buffer,
                            "{:>5} {:>5} │{}",
                            style(change.old_index().unwrap()).cyan().dim(),
                            "",
                            style("-").red(),
                        )?;
                        for &(emphasized, change) in change.values() {
                            if emphasized {
                                write!(buffer, "{}", style(change).red().underlined())?;
                            } else {
                                write!(buffer, "{}", style(change).red())?;
                            }
                        }
                    }
                    ChangeTag::Equal => {
                        write!(
                            buffer,
                            "{:>5} {:>5} │ ",
                            style(change.old_index().unwrap()).cyan().dim(),
                            style(change.new_index().unwrap()).cyan().dim().bold(),
                        )?;
                        for &(_, change) in change.values() {
                            write!(buffer, "{}", style(change).dim())?;
                        }
                    }
                }
                if change.missing_newline() {
                    writeln!(buffer,)?;
                }
            }
        }
    }

    if !has_changes {
        writeln!(
            buffer,
            "{:>5} {:>5} │{}",
            "",
            style("-").dim(),
            style(" snapshots are matching").cyan(),
        )?;
    }

    writeln!(buffer, "────────────┴{:─^1$}", "", width.saturating_sub(13),)?;

    Ok(())
}
