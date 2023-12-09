use std::io::Write;

use anyhow::Context;
use run_script::types::ScriptOptions;

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
    outcome: StepCommandOutcome,
    save_at: String,
}

#[derive(Debug, serde::Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
enum StepCommandOutcome {
    Success,
    Failure,
}

fn main() -> Result<(), anyhow::Error> {
    let tutorial_manifest = fs_err::read_to_string("tutorial.yml")
        .context("Failed to open the tutorial manifest file. Are you in the right directory?")?;
    let deserializer = serde_yaml::Deserializer::from_str(&tutorial_manifest);
    let tutorial_manifest: TutorialManifest = serde_path_to_error::deserialize(deserializer)
        .context("Failed to parse the tutorial manifest file")?;

    clean_up();

    // Boostrap the project
    println!("Running bootstrap script");
    let (code, output, error) = run_script::run(
        &tutorial_manifest.bootstrap,
        &Default::default(),
        &ScriptOptions::new(),
    )
    .expect("Failed to run the boostrap script");
    if code != 0 {
        eprintln!("Failed to run the boostrap script");
        eprintln!("Exit Code: {}", code);
        eprintln!("Output: {}", output);
        eprintln!("Error: {}", error);
        std::process::exit(1);
    }

    // Apply the patches
    let mut previous_dir = tutorial_manifest.starter_project_folder;
    for (i, step) in tutorial_manifest.steps.iter().enumerate() {
        println!("Applying patch: {}", step.patch);
        let next_dir = patch_directory_name(i);
        let (code, output, error) = run_script::run(
            &format!(
                r#"cp -r {previous_dir} {next_dir}
cd {next_dir} && patch -p1 < ../{} && git add . && git commit -am "First commit""#,
                step.patch
            ),
            &Default::default(),
            &ScriptOptions::new(),
        )
        .expect("Failed to run patch");
        if code != 0 {
            eprintln!("Failed to run patch");
            eprintln!("Exit Code: {}", code);
            eprintln!("Output: {}", output);
            eprintln!("Error: {}", error);
            std::process::exit(1);
        }
        previous_dir = next_dir;
    }

    for (i, step) in tutorial_manifest.steps.iter().enumerate() {
        for command in &step.commands {
            println!(
                "Running command for patch `{}`: {}",
                step.patch, command.command
            );
            let patch_dir = patch_directory_name(i);
            let (code, output, error) = run_script::run(
                &format!(r#"cd {patch_dir} && {}"#, command.command),
                &Default::default(),
                &ScriptOptions::new(),
            )
            .expect("Failed to run command");

            if command.outcome == StepCommandOutcome::Success && code != 0 {
                eprintln!("Failed to run command which should have succeeded");
                eprintln!("Exit Code: {}", code);
                eprintln!("Output: {}", output);
                eprintln!("Error: {}", error);
                std::process::exit(1);
            } else if command.outcome == StepCommandOutcome::Failure && code == 0 {
                eprintln!("Command succeeded when it should have failed");
                eprintln!("Exit Code: {}", code);
                eprintln!("Output: {}", output);
                eprintln!("Error: {}", error);
                std::process::exit(1);
            }

            let mut file = fs_err::File::create(&command.save_at).expect("Failed to create file");
            let contents = match command.outcome {
                StepCommandOutcome::Success => output,
                StepCommandOutcome::Failure => error,
            };
            file.write_all(contents.as_bytes())
                .expect("Failed to write to file");
        }
    }

    Ok(())
}

fn patch_directory_name(patch_index: usize) -> String {
    format!("{:02}", patch_index + 1)
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
