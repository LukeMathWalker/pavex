mod embedded;
mod external;
mod snippet;
mod source_range;

pub use embedded::{FileKind, extract_embedded_snippets, extract_embedded_snippets_from_content};
pub use external::{
    ExternalSnippetSpec, extract_external_snippet, extract_external_snippet_from_content,
};
pub use snippet::{Snippet, SnippetLine, SnippetName};
pub use source_range::SourceRange;
