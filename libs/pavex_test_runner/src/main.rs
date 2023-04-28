use std::path::PathBuf;

use console::{style, Key};

use pavex_test_runner::{get_test_name, get_ui_test_directories, print_changeset};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    miette::set_hook(Box::new(move |_| {
        let config = miette::MietteHandlerOpts::new();
        Box::new(config.build())
    }))
    .unwrap();
    let test_folder = workspace_root()?.join("pavex_cli/tests/ui_tests");
    let terminal = console::Term::stdout();
    for ui_test_dir in get_ui_test_directories(&test_folder) {
        assert!(ui_test_dir.as_path().metadata()?.is_dir());
        let test_name = get_test_name(&test_folder, &ui_test_dir);
        let expectations_dir = ui_test_dir.as_path().join("expectations");
        for file in fs_err::read_dir(&expectations_dir)? {
            let file = file?;
            let file_name = file.file_name().to_string_lossy().to_string();
            if let Some(expected_filename) = file_name.strip_suffix(".snap") {
                let actual_snapshot = fs_err::read_to_string(file.path())?;
                let expected_path = expectations_dir.join(expected_filename);
                let expected_snapshot = match fs_err::read_to_string(&expected_path) {
                    Ok(s) => s,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => "".to_string(),
                    Err(e) => return Err(e.into()),
                };

                match review_snapshot(
                    &terminal,
                    &test_name,
                    expected_filename,
                    &expected_snapshot,
                    &actual_snapshot,
                )? {
                    Decision::Accept => {
                        fs_err::rename(file.path(), &expected_path)?;
                    }
                    Decision::Reject => {
                        fs_err::remove_file(file.path())?;
                    }
                    Decision::Skip => continue,
                }
            }
        }
    }

    Ok(())
}

fn review_snapshot(
    terminal: &console::Term,
    test_name: &str,
    snapshot_name: &str,
    expected_snapshot: &str,
    actual_snapshot: &str,
) -> std::io::Result<Decision> {
    terminal.clear_screen()?;
    println!(
        "{}{}\n{}{}",
        style("Test name: ").bold(),
        style((&test_name).to_string()).yellow().bold(),
        style("Snapshot name: ").bold(),
        style((&snapshot_name).to_string()).green().bold(),
    );
    print_changeset(expected_snapshot, actual_snapshot);

    prompt(terminal)
}

fn prompt(terminal: &console::Term) -> std::io::Result<Decision> {
    println!();
    println!(
        "  {} accept     {}",
        style("a").green().bold(),
        style("keep the new snapshot").dim()
    );
    println!(
        "  {} reject     {}",
        style("r").red().bold(),
        style("keep the old snapshot").dim()
    );
    println!(
        "  {} skip       {}",
        style("s").yellow().bold(),
        style("keep both for now").dim()
    );
    loop {
        match terminal.read_key()? {
            Key::Char('a') | Key::Enter => return Ok(Decision::Accept),
            Key::Char('r') | Key::Escape => return Ok(Decision::Reject),
            Key::Char('s') | Key::Char(' ') => return Ok(Decision::Skip),
            _ => {}
        }
    }
}

enum Decision {
    Accept,
    Reject,
    Skip,
}

/// Retrieve the root directory of the current workspace.
fn workspace_root() -> Result<PathBuf, anyhow::Error> {
    #[derive(serde::Deserialize)]
    struct LocateProject {
        root: PathBuf,
    }

    let output = std::process::Command::new("cargo")
        .arg("locate-project")
        .arg("--workspace")
        .output()?;
    let json: LocateProject = serde_json::from_slice(&output.stdout)?;
    Ok(json.root.parent().unwrap().to_path_buf())
}
