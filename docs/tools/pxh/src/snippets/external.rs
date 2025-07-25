use camino::{Utf8Path, Utf8PathBuf};

use crate::snippets::embedded::FileKind;

use super::{SnippetName, SourceRange};

#[derive(Debug)]
pub struct ExternalSnippetSpec {
    /// The name of this snippet.
    ///
    /// The snippet will be saved as `<name>.snap`, in the root of the example/tutorial folder.
    pub name: SnippetName,
    /// The path to the source file we want to extract the snippet from.
    ///
    /// It's expected to be relative to the root of the example/tutorial folder.
    pub source_path: Utf8PathBuf,
    /// The ranges of lines we want to extract from the source file.
    pub ranges: Vec<SourceRange>,
    /// Which lines should be highlighted in the snippet.
    /// The line numbers are relative to the start of the snippet, **not** to the
    /// line numbers in the original source file.
    pub hl_lines: Vec<usize>,
}

pub fn extract_external_snippet(
    base_path: &Utf8Path,
    title: Option<&str>,
    spec: &ExternalSnippetSpec,
) -> Result<String, anyhow::Error> {
    println!("Extracting external snippet: {}", spec.name);
    let source_path = base_path.join(&spec.source_path);
    let content = fs_err::read_to_string(&source_path)?;
    let Some(kind) = FileKind::from_path(&source_path) else {
        anyhow::bail!("Unsupported file extension for {source_path}");
    };
    extract_external_snippet_from_content(&content, title, kind, spec)
}

pub fn extract_external_snippet_from_content(
    content: &str,
    title: Option<&str>,
    kind: FileKind,
    spec: &ExternalSnippetSpec,
) -> Result<String, anyhow::Error> {
    use std::fmt::Write as _;

    let mut extracted_snippet = match kind {
        FileKind::Rust => "```rust",
        FileKind::Toml => "```toml",
    }
    .to_string();

    if let Some(title) = title {
        write!(&mut extracted_snippet, " title=\"{}\"", title)?;
    }

    if !spec.hl_lines.is_empty() {
        write!(&mut extracted_snippet, " hl_lines=\"").unwrap();
        for (idx, line) in spec.hl_lines.iter().enumerate() {
            if idx > 0 {
                write!(&mut extracted_snippet, " ").unwrap();
            }
            write!(&mut extracted_snippet, "{}", line).unwrap();
        }
        write!(&mut extracted_snippet, "\"").unwrap();
    }

    // End of the header
    extracted_snippet.push('\n');

    {
        let extracted_block = spec
            .ranges
            .iter()
            .map(|range| range.extract_lines(&content))
            .collect::<Vec<_>>();

        let mut previous_leading_whitespaces = 0;
        for (i, block) in extracted_block.iter().enumerate() {
            let current_leading_whitespaces = block
                .lines()
                .next()
                .map(|l| l.chars().take_while(|c| c.is_whitespace()).count())
                .unwrap_or(0);

            let add_ellipsis = if i > 0 {
                true
            } else {
                let not_from_the_start = match &spec.ranges[i] {
                    SourceRange::Range(r) => r.start > 0,
                    SourceRange::RangeInclusive(r) => *r.start() > 0,
                    SourceRange::RangeFrom(r) => r.start > 0,
                    SourceRange::RangeFull => false,
                };
                not_from_the_start
            };

            if add_ellipsis {
                let comment_leading_whitespaces =
                    if current_leading_whitespaces > previous_leading_whitespaces {
                        current_leading_whitespaces
                    } else {
                        previous_leading_whitespaces
                    };
                let indent = " ".repeat(comment_leading_whitespaces);
                if i != 0 {
                    extracted_snippet.push('\n');
                }
                match kind {
                    FileKind::Rust => {
                        writeln!(&mut extracted_snippet, "{indent}// [...]").unwrap();
                    }
                    FileKind::Toml => {
                        writeln!(&mut extracted_snippet, "{indent}# [...]").unwrap();
                    }
                }
            }
            extracted_snippet.push_str(&block);
            previous_leading_whitespaces = block
                .lines()
                .last()
                .map(|l| l.chars().take_while(|c| c.is_whitespace()).count())
                .unwrap_or(0);
        }

        write!(&mut extracted_snippet, "\n```")?;

        Ok(extracted_snippet)
    }
}
