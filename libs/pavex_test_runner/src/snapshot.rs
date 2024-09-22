use std::io::ErrorKind;
use std::path::PathBuf;
use std::time::Duration;

use console::style;
use regex::Captures;
use similar::{Algorithm, ChangeTag, TextDiff};

fn term_width() -> usize {
    console::Term::stdout().size().1 as usize
}

pub(crate) struct SnapshotTest {
    expectation_path: PathBuf,
    app_name_with_hash: String,
}

impl SnapshotTest {
    pub fn new(expectation_path: PathBuf, app_name_with_hash: String) -> Self {
        Self {
            expectation_path,
            app_name_with_hash,
        }
    }

    pub fn verify(&self, actual: &str) -> Result<(), ()> {
        // All test crates have a hash suffix in their name to avoid name collisions.
        // We remove this hash to make the snapshots more stable and readable.
        let actual = actual.replace(&self.app_name_with_hash, "app");

        let expected = match fs_err::read_to_string(&self.expectation_path) {
            Ok(s) => s,
            Err(e) if e.kind() == ErrorKind::NotFound => "".into(),
            outcome @ Err(_) => {
                outcome.expect("Failed to load the expected value for a snapshot test")
            }
        };
        let trimmed_expected = expected.trim();
        let actual = actual.trim();

        // Replace all line endings with `\n` to make sure that the snapshots are cross-platform.
        let trimmed_expected = trimmed_expected.replace("\r\n", "\n");
        let actual = actual.replace("\r\n", "\n");

        // Path normalization for Windows, which uses `\` instead of `/` as path separator.
        static RE: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
            regex::Regex::new(r#"(?<prefix>\[\[36;1;4m)(?<path>.*)"#).unwrap()
        });
        let normalizer = |c: &Captures| {
            let prefix = c.name("prefix").unwrap().as_str();
            let path = c.name("path").unwrap().as_str().replace("\\", "/");
            format!("{prefix}{path}",)
        };
        let trimmed_expected = RE.replace_all(&trimmed_expected, normalizer);
        let actual = RE.replace_all(&actual, normalizer);

        let expectation_directory = self.expectation_path.parent().unwrap();
        let last_snapshot_path = expectation_directory.join(format!(
            "{}.snap",
            self.expectation_path.file_name().unwrap().to_string_lossy()
        ));

        if trimmed_expected != actual {
            print_changeset(&trimmed_expected, &actual);
            fs_err::write(last_snapshot_path, actual.as_ref())
                .expect("Failed to save the actual value for a failed snapshot test");
            Err(())
        } else {
            let _ = fs_err::remove_file(last_snapshot_path);
            Ok(())
        }
    }
}

pub fn print_changeset(old: &str, new: &str) {
    let width = term_width();
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Patience)
        .timeout(Duration::from_millis(500))
        .diff_lines(old, new);
    println!("{:─^1$}", "", width);

    if !old.is_empty() {
        println!("{}", style("-old snapshot").red());
        println!("{}", style("+new results").green());
    } else {
        println!("{}", style("+new results").green());
    }

    println!("────────────┬{:─^1$}", "", width.saturating_sub(13));
    let mut has_changes = false;
    for (idx, group) in diff.grouped_ops(4).iter().enumerate() {
        if idx > 0 {
            println!("┈┈┈┈┈┈┈┈┈┈┈┈┼{:┈^1$}", "", width.saturating_sub(13));
        }
        for op in group {
            for change in diff.iter_inline_changes(op) {
                match change.tag() {
                    ChangeTag::Insert => {
                        has_changes = true;
                        print!(
                            "{:>5} {:>5} │{}",
                            "",
                            style(change.new_index().unwrap()).cyan().dim().bold(),
                            style("+").green(),
                        );
                        for &(emphasized, change) in change.values() {
                            if emphasized {
                                print!("{}", style(change).green().underlined());
                            } else {
                                print!("{}", style(change).green());
                            }
                        }
                    }
                    ChangeTag::Delete => {
                        has_changes = true;
                        print!(
                            "{:>5} {:>5} │{}",
                            style(change.old_index().unwrap()).cyan().dim(),
                            "",
                            style("-").red(),
                        );
                        for &(emphasized, change) in change.values() {
                            if emphasized {
                                print!("{}", style(change).red().underlined());
                            } else {
                                print!("{}", style(change).red());
                            }
                        }
                    }
                    ChangeTag::Equal => {
                        print!(
                            "{:>5} {:>5} │ ",
                            style(change.old_index().unwrap()).cyan().dim(),
                            style(change.new_index().unwrap()).cyan().dim().bold(),
                        );
                        for &(_, change) in change.values() {
                            print!("{}", style(change).dim());
                        }
                    }
                }
                if change.missing_newline() {
                    println!();
                }
            }
        }
    }

    if !has_changes {
        println!(
            "{:>5} {:>5} │{}",
            "",
            style("-").dim(),
            style(" snapshots are matching").cyan(),
        );
    }

    println!("────────────┴{:─^1$}", "", width.saturating_sub(13),);
}
