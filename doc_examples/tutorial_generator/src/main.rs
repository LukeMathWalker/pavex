use anyhow::Context;
use run_script::types::ScriptOptions;

#[derive(Debug, serde::Deserialize)]
struct TutorialManifest {
    boostrap: String,
    starter_project_folder: String,
    steps: Vec<Step>,
}

#[derive(Debug, serde::Deserialize)]
struct Step {
    patch: String,
}

fn main() -> Result<(), anyhow::Error> {
    let tutorial_manifest = fs_err::read_to_string("tutorial.yml")
        .context("Failed to open the tutorial manifest file. Are you in the right directory?")?;
    let deserializer = serde_yaml::Deserializer::from_str(&tutorial_manifest);
    let tutorial_manifest: TutorialManifest = serde_path_to_error::deserialize(deserializer)
        .context("Failed to parse the tutorial manifest file")?;

    clean_up();

    // Boostrap the project
    let (code, output, error) = run_script::run(
        &tutorial_manifest.boostrap,
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
        let next_dir = format!("{:02}", i);
        let (code, output, error) = run_script::run(
            &format!(
                r#"cp -r {previous_dir} {next_dir}
cd {next_dir} && patch -p1 < ../{}"#,
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

    Ok(())
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
            !file_name.ends_with(".patch") && file_name != "tutorial.yml"
        })
        .for_each(|entry| {
            fs_err::remove_dir_all(entry.path()).expect("Failed to remove a file or directory")
        });
}
