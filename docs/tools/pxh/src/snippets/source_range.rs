use std::str::FromStr;

use anyhow::Context;

/// Enum representing a range of lines in a source file.
#[derive(Debug)]
pub enum SourceRange {
    Range(std::ops::Range<usize>),
    RangeInclusive(std::ops::RangeInclusive<usize>),
    RangeFrom(std::ops::RangeFrom<usize>),
    RangeFull,
}

impl SourceRange {
    pub fn extract_lines(&self, source: &str) -> String {
        let mut lines = source.lines();
        let iterator: Box<dyn Iterator<Item = &str>> = match self {
            SourceRange::Range(range) => Box::new(
                lines
                    .by_ref()
                    .skip(range.start)
                    .take(range.end - range.start),
            ),
            SourceRange::RangeInclusive(range) => Box::new(
                lines
                    .by_ref()
                    .skip(*range.start())
                    .take(*range.end() - *range.start() + 1),
            ),
            SourceRange::RangeFrom(range) => Box::new(lines.by_ref().skip(range.start)),
            SourceRange::RangeFull => Box::new(lines.by_ref()),
        };
        let mut buffer = String::new();
        for (idx, line) in iterator.enumerate() {
            if idx > 0 {
                buffer.push('\n');
            }
            buffer.push_str(line);
        }
        buffer
    }
}

impl FromStr for SourceRange {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == ".." {
            return Ok(SourceRange::RangeFull);
        } else if s.starts_with("..") {
            anyhow::bail!(
                "Ranges must always specify a starting line. Invalid range: `{}`",
                s
            );
        }
        if s.contains("..=") {
            let mut parts = s.split("..=");
            let start: usize = parts
                .next()
                .unwrap()
                .parse()
                .context("Range start line must be a valid number")?;
            match parts.next() {
                Some(end) => {
                    let end: usize = end
                        .parse()
                        .context("Range end line must be a valid number")?;
                    Ok(SourceRange::RangeInclusive(start..=end))
                }
                None => Ok(SourceRange::RangeFrom(start..)),
            }
        } else {
            let mut parts = s.split("..");
            let start: usize = parts
                .next()
                .unwrap()
                .parse()
                .context("Range start line must be a valid number")?;
            match parts.next() {
                Some(s) if s.is_empty() => Ok(SourceRange::RangeFrom(start..)),
                None => Ok(SourceRange::RangeFrom(start..)),
                Some(end) => {
                    let end: usize = end
                        .parse()
                        .context("Range end line must be a valid number")?;
                    Ok(SourceRange::Range(start..end))
                }
            }
        }
    }
}
