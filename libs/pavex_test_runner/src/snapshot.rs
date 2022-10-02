use std::io::ErrorKind;
use std::path::PathBuf;
use std::time::Duration;

use console::style;
use similar::{Algorithm, ChangeTag, TextDiff};

fn term_width() -> usize {
    console::Term::stdout().size().1 as usize
}

pub(crate) struct SnapshotTest {
    expectation_path: PathBuf,
}

impl SnapshotTest {
    pub fn new(expectation_path: PathBuf) -> Self {
        Self { expectation_path }
    }

    pub fn verify(&self, actual: &str) -> Result<(), ()> {
        let expected = match fs_err::read_to_string(&self.expectation_path) {
            Ok(s) => s,
            Err(e) if e.kind() == ErrorKind::NotFound => "".into(),
            outcome @ Err(_) => {
                outcome.expect("Failed to load the expected value for a snapshot test")
            }
        };
        let trimmed_expected = expected.trim();
        let actual = actual.trim();

        let expectation_directory = self.expectation_path.parent().unwrap();
        let last_snapshot_path = expectation_directory.join(format!(
            "{}.snap",
            self.expectation_path.file_name().unwrap().to_string_lossy()
        ));

        if trimmed_expected != actual {
            print_changeset(expected.trim(), actual);
            fs_err::write(last_snapshot_path, actual)
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
