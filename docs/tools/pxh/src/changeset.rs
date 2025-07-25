use console::style;
use similar::{Algorithm, ChangeTag, TextDiff};
use std::time::Duration;

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
