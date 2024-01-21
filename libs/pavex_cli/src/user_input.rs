use anyhow::{anyhow, Context};
use std::io::{BufRead, Write};

/// Prompt the user for confirmation.
pub fn confirm(question: &str, default: bool) -> Result<bool, anyhow::Error> {
    write!(std::io::stdout().lock(), "{question} ")?;
    let _ = std::io::stdout().flush();
    let input = read_line()?;

    let r = match &*input.to_lowercase() {
        "y" | "yes" => true,
        "n" | "no" => false,
        "" => default,
        _ => false,
    };

    writeln!(std::io::stdout().lock())?;

    Ok(r)
}

/// Prompt the user for input, without a default value.
pub fn mandatory_question(question: &str) -> anyhow::Result<String> {
    let mut input = "".to_string();
    while input.is_empty() {
        writeln!(std::io::stdout().lock(), "{question}")?;
        let _ = std::io::stdout().flush();
        input = read_line()?;
        writeln!(std::io::stdout().lock())?;
    }
    Ok(input)
}

fn read_line() -> Result<String, anyhow::Error> {
    let stdin = std::io::stdin();
    let stdin = stdin.lock();
    let mut lines = stdin.lines();
    let lines = lines.next().transpose()?;
    match lines {
        None => Err(anyhow!("No lines found from stdin")),
        Some(v) => Ok(v),
    }
    .context("Unable to read from stdin for confirmation")
}
