use std::{collections::HashMap, sync::LazyLock};

use camino::Utf8Path;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct Snippet {
    pub name: SnippetName,
    lines: Vec<SnippetLine>,
    pub begin_with_ellipsis: bool,
    pub annotations: HashMap<usize, Vec<(String, usize)>>, // line -> [(annotation_text, number)]
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SnippetName(String);

#[derive(Debug, Clone)]
pub struct SnippetLine {
    pub line_number: usize,
    pub content: String,
    pub followed_by_ellipsis: bool,
    pub highlighted: bool,
}

impl SnippetName {
    pub fn new(name: String) -> Result<SnippetName, anyhow::Error> {
        if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            Err(anyhow::anyhow!("Invalid snippet name: {}", name))
        } else {
            Ok(Self(name))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SnippetName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialEq<&str> for SnippetName {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl Snippet {
    pub fn new(name: SnippetName) -> Self {
        Self {
            name,
            lines: Vec::new(),
            annotations: HashMap::new(),
            begin_with_ellipsis: false,
        }
    }

    pub fn lines(&self) -> &[SnippetLine] {
        &self.lines
    }

    pub fn lines_mut(&mut self) -> &mut [SnippetLine] {
        &mut self.lines
    }

    pub fn push_line(&mut self, snippet_line: SnippetLine) {
        static ANNOTATION_REGEX: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r" px:[a-zA-Z0-9_]?:ann:(?<number>\d+) ").unwrap());

        let SnippetLine {
            line_number,
            content,
            followed_by_ellipsis,
            highlighted,
        } = snippet_line;
        if content.trim_end().is_empty() {
            // We don't want empty lines at the beginning of the snippet,
            // nor consecutive empty lines.
            let must_omit = match self.lines.last() {
                Some(previous) => previous.content.trim().is_empty(),
                None => true,
            };
            if must_omit {
                return;
            }
        }

        let content = ANNOTATION_REGEX.replace_all(&content, " ($number)! ");
        let contains_annotation = matches!(content, std::borrow::Cow::Owned(_));
        self.lines.push(SnippetLine {
            line_number,
            content: content.into_owned(),
            followed_by_ellipsis,
            // Automatically highlight lines containing annotations
            highlighted: highlighted || contains_annotation,
        });
    }

    pub fn render(&self, file_path: &Utf8Path, title: Option<String>) -> String {
        let mut output = String::new();

        // Determine language from file extension
        let lang = match file_path.extension() {
            Some("rs") => "rust",
            Some("toml") => "toml",
            Some("yaml") | Some("yml") => "yaml",
            Some("json") => "json",
            _ => "text",
        };

        // Build the code fence header
        output.push_str(&format!("```{lang}"));

        if let Some(title) = title {
            output.push_str(&format!(" title=\"{title}\""));
        }

        // Calculate highlight lines based on output position
        let mut highlight_output_lines = Vec::new();
        let mut output_line_num = if self.begin_with_ellipsis { 2 } else { 1 };

        for line in &self.lines {
            if line.highlighted {
                highlight_output_lines.push(output_line_num);
            }
            output_line_num += if line.followed_by_ellipsis { 2 } else { 1 };
        }

        if !highlight_output_lines.is_empty() {
            output.push_str(" hl_lines=\"");
            for (i, line) in highlight_output_lines.iter().enumerate() {
                if i > 0 {
                    output.push(' ');
                }
                output.push_str(&line.to_string());
            }
            output.push('"');
        }

        output.push('\n');

        if self.begin_with_ellipsis {
            match lang {
                "rust" => output.push_str("// [...]\n"),
                "toml" | "yaml" => output.push_str("# [...]\n"),
                _ => output.push_str("[...]\n"),
            }
        }

        // Calculate minimum indentation (excluding skipped lines and empty lines)
        let min_indent = self
            .lines
            .iter()
            .filter(|line| !line.content.trim().is_empty())
            .map(|line| {
                line.content
                    .chars()
                    .take_while(|c| c.is_whitespace())
                    .count()
            })
            .min()
            .unwrap_or(0);

        // Render the snippet content
        // Remove empty lines until we reach the first non-empty one
        for (index, line) in self.lines.iter().enumerate() {
            let SnippetLine {
                line_number,
                content,
                followed_by_ellipsis,
                highlighted: _,
            } = line;
            // Process annotations in the line
            let mut processed_content = content.clone();
            if let Some(annotations) = self.annotations.get(line_number) {
                for (ann_text, ann_num) in annotations {
                    processed_content =
                        processed_content.replace(ann_text, &format!("/* ({})! */", ann_num));
                }
            }

            // Normalize indentation by removing the minimum indent
            let normalized_content = if !processed_content.trim().is_empty() {
                &processed_content[min_indent..]
            } else {
                &processed_content
            };

            output.push_str(normalized_content);
            output.push('\n');

            if let Some(next_line) = self.lines.get(index + 1)
                && *followed_by_ellipsis
            {
                let next_line_indent = next_line
                    .content
                    .chars()
                    .take_while(|c| c.is_whitespace())
                    .count();
                let next_starts_with_closing_brace =
                    next_line.content.trim_start().starts_with('}');
                let current_indent = if next_starts_with_closing_brace {
                    next_line_indent + 4
                } else {
                    next_line_indent
                };
                let ellipsis_indent = if current_indent >= min_indent {
                    " ".repeat(current_indent - min_indent)
                } else {
                    String::new()
                };
                match lang {
                    "rust" => output.push_str(&format!("{}// [...]\n", ellipsis_indent)),
                    "toml" | "yaml" => output.push_str(&format!("{}# [...]\n", ellipsis_indent)),
                    _ => output.push_str(&format!("{}[...]\n", ellipsis_indent)),
                }
            }
        }

        output.push_str("```");
        output
    }
}
