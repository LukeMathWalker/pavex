use crate::snippets::SnippetLine;

use super::{Snippet, SnippetName};
use anyhow::Result;
use camino::Utf8Path;
use fs_err as fs;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileKind {
    Rust,
    Toml,
}

impl FileKind {
    pub fn from_path(path: &Utf8Path) -> Option<FileKind> {
        match path.extension() {
            Some("rs") => Some(FileKind::Rust),
            Some("toml") => Some(FileKind::Toml),
            _ => None,
        }
    }
}

/// Extract snippets that have been embedded into a file using comments.
pub fn extract_embedded_snippets(file_path: &Utf8Path) -> Result<Vec<Snippet>> {
    let content = fs::read_to_string(file_path)?;
    let Some(kind) = FileKind::from_path(file_path) else {
        anyhow::bail!("Unsupported file extension for {file_path}");
    };
    extract_embedded_snippets_from_content(&content, kind)
}

/// Extract snippets that have been embedded using comments.
pub fn extract_embedded_snippets_from_content(
    content: &str,
    kind: FileKind,
) -> Result<Vec<Snippet>> {
    struct ParsingState {
        skipping: bool,
    }

    let mut snippets: HashMap<SnippetName, Snippet> = HashMap::new();
    let mut active_snippets: HashMap<SnippetName, ParsingState> = HashMap::new();
    let mut reached_content = false;

    let lines: Vec<&str> = content.lines().collect();
    for (idx, line) in lines.iter().enumerate() {
        if let Some(FileMarker { name }) = FileMarker::extract(line, kind)? {
            if reached_content {
                anyhow::bail!("File-wide markers must be placed before content")
            }
            let mut snippet = Snippet::new(name.clone());
            if reached_content {
                snippet.begin_with_ellipsis = true;
            }
            snippets.insert(name.clone(), snippet);
            active_snippets.insert(name, ParsingState { skipping: false });
        } else if let Some(marker) = extract_region_marker(line, kind)? {
            match marker {
                RegionMarker::SkipStart { name } => {
                    if let Some(name) = name {
                        active_snippets.get_mut(&name).unwrap().skipping = true;
                    } else {
                        for active in active_snippets.values_mut() {
                            active.skipping = true;
                        }
                    }
                }
                RegionMarker::SkipEnd { name } => {
                    if let Some(name) = name {
                        active_snippets.get_mut(&name).unwrap().skipping = false;
                    } else {
                        for active in active_snippets.values_mut() {
                            active.skipping = false;
                        }
                    }
                }
                RegionMarker::SnippetStart { name } => {
                    snippets.entry(name.clone()).or_insert_with(|| {
                        let mut snippet = Snippet::new(name.clone());
                        if reached_content {
                            snippet.begin_with_ellipsis = true;
                        }
                        snippet
                    });
                    active_snippets.insert(name.clone(), ParsingState { skipping: false });
                }
                RegionMarker::SnippetEnd { name } => {
                    active_snippets.get_mut(&name).unwrap().skipping = true;
                }
            }
        } else {
            let (line, trailing_markers, has_annotations) = extract_trailing_markers(line, kind)?
                .unwrap_or_else(|| (line.to_string(), Vec::new(), false));

            if !line.is_empty() {
                reached_content = true;
            }

            for (name, state) in &active_snippets {
                let skip = trailing_markers.iter().any(|m| {
                    if let TrailingMarker::Skip {
                        name: skipping_name,
                    } = m
                    {
                        skipping_name.is_none() || skipping_name.as_ref() == Some(name)
                    } else {
                        false
                    }
                });
                if !state.skipping && !skip {
                    let snippet = snippets.get_mut(name).unwrap();
                    let highlighted = trailing_markers.iter().any(|m| {
                        if let TrailingMarker::Highlight {
                            name: highlight_name,
                        } = m
                        {
                            highlight_name.is_none() || highlight_name.as_ref() == Some(name)
                        } else {
                            false
                        }
                    }) | has_annotations;
                    snippet.push_line(SnippetLine {
                        line_number: idx + 1,
                        content: line.clone(),
                        followed_by_ellipsis: false,
                        highlighted,
                    });
                } else {
                    let snippet = snippets.get_mut(name).unwrap();
                    if let Some(last) = snippet.lines_mut().last_mut() {
                        last.followed_by_ellipsis = true;
                    } else {
                        snippet.begin_with_ellipsis = true;
                    }
                }
            }
        }
    }
    Ok(snippets.into_values().collect())
}

pub struct FileMarker {
    name: SnippetName,
}

impl FileMarker {
    pub fn extract(line: &str, kind: FileKind) -> Result<Option<Self>, anyhow::Error> {
        let file_comment_marker = match kind {
            FileKind::Rust => "//!",
            FileKind::Toml => "#!",
        };
        let line = line.trim();
        let Some(comment_starts_at) = line.find(file_comment_marker) else {
            return Ok(None);
        };
        let comment = line[comment_starts_at + file_comment_marker.len()..].trim();
        let Some(command) = comment.strip_prefix("px:") else {
            return Ok(None);
        };
        let name = SnippetName::new(command.to_string())?;
        if comment_starts_at != 0 {
            anyhow::bail!("File marker must be at the beginning of the line they're on");
        }
        Ok(Some(Self { name }))
    }
}

pub enum RegionMarker {
    // `px::skip:start` or `px:<snippet_name>:skip:start`
    SkipStart { name: Option<SnippetName> },
    // `px::skip:end` or `px:<snippet_name>:skip:end`
    SkipEnd { name: Option<SnippetName> },
    // `px:<snippet_name>:start`
    SnippetStart { name: SnippetName },
    // `px:<snippet_name>:end`
    SnippetEnd { name: SnippetName },
}

fn extract_region_marker(
    line: &str,
    kind: FileKind,
) -> Result<Option<RegionMarker>, anyhow::Error> {
    let line_comment_marker = match kind {
        FileKind::Rust => "//",
        FileKind::Toml => "#",
    };
    let line = line.trim();
    let Some(comment_starts_at) = line.find(line_comment_marker) else {
        return Ok(None);
    };
    if comment_starts_at != 0 {
        // Region-marker must appear at the beginning of their line.
        return Ok(None);
    }
    let comment = line[comment_starts_at + line_comment_marker.len()..].trim();
    let Some(command) = comment.strip_prefix("px:") else {
        return Ok(None);
    };
    let command_parts: Vec<_> = command.split(':').collect();

    let region_marker = match command_parts.as_slice() {
        [name, "skip", "start"] => RegionMarker::SkipStart {
            name: if name.is_empty() {
                None
            } else {
                Some(SnippetName::new(name.to_string())?)
            },
        },
        [name, "skip", "end"] => RegionMarker::SkipEnd {
            name: if name.is_empty() {
                None
            } else {
                Some(SnippetName::new(name.to_string())?)
            },
        },
        [name, "start"] => RegionMarker::SnippetStart {
            name: SnippetName::new(name.to_string())?,
        },
        [name, "end"] => RegionMarker::SnippetEnd {
            name: SnippetName::new(name.to_string())?,
        },
        _ => anyhow::bail!("Unknown region marker (`{command}`) in line (`{line}`)"),
    };
    Ok(Some(region_marker))
}

pub enum TrailingMarker {
    Skip { name: Option<SnippetName> },
    Highlight { name: Option<SnippetName> },
}

/// Parse trailing markers from a line of code.
///
/// If successful, it returns a modified line with trailing markers stripped away from it as well as the parsed
/// markers.
/// It will preserve annotation markers, since they must appear in the extracted snippet.
fn extract_trailing_markers(
    line: &str,
    kind: FileKind,
) -> Result<Option<(String, Vec<TrailingMarker>, bool)>, anyhow::Error> {
    let line_comment_marker = match kind {
        FileKind::Rust => "//",
        FileKind::Toml => "#",
    };
    let Some((line_content, trailing_comment)) = line.rsplit_once(line_comment_marker) else {
        return Ok(None);
    };
    let line_content = line_content.trim_end();
    let comment = trailing_comment.trim();
    let raw_commands = comment.split_whitespace().collect::<Vec<_>>();
    if !raw_commands.iter().any(|raw| raw.starts_with("px:")) {
        // It's a regular trailing comment, nothing to do.
        return Ok(None);
    }

    let mut parsed_commands = Vec::with_capacity(raw_commands.len());
    let mut annotations = Vec::new();
    for raw_command in raw_commands {
        let Some(command) = raw_command.strip_prefix("px:") else {
            anyhow::bail!("Expected trailing px: command, found `{raw_command}` on line: `{line}`")
        };
        let command_parts: Vec<_> = command.split(':').collect();
        match command_parts.as_slice() {
            [name, "skip"] => {
                let name = if name.is_empty() {
                    None
                } else {
                    Some(SnippetName::new(name.to_string())?)
                };
                parsed_commands.push(TrailingMarker::Skip { name });
            }
            [name, "hl"] => {
                let name = if name.is_empty() {
                    None
                } else {
                    Some(SnippetName::new(name.to_string())?)
                };
                parsed_commands.push(TrailingMarker::Highlight { name });
            }
            [_name, "ann", number] => {
                let number: u8 = number.parse()?;
                annotations.push(format!("({number})!"));
            }
            _ => anyhow::bail!("Unexpected trailing command `{raw_command}` on line: `{line}`"),
        }
    }

    // Add parsed annotations back in.
    let mut reconstructed_line = line_content.to_string();
    let has_annotations = !annotations.is_empty();
    if has_annotations {
        reconstructed_line.push_str(" // ");
        for annotation in annotations {
            reconstructed_line.push_str(&annotation);
            reconstructed_line.push(' ');
        }
        reconstructed_line = reconstructed_line.trim_end().to_string();
    }
    Ok(Some((reconstructed_line, parsed_commands, has_annotations)))
}
