use std::fmt::Display;
use std::path::Path;

use miette::{Diagnostic, LabeledSpan, NamedSource, SourceCode, SourceOffset, SourceSpan};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{ExprMethodCall, Stmt};

use pavex_builder::Location;

pub struct CompilerDiagnosticBuilder {
    source_code: NamedSource,
    labels: Option<Vec<LabeledSpan>>,
    help: Option<String>,
    error_source: anyhow::Error,
    related_errors: Option<Vec<CompilerDiagnostic>>,
}

impl CompilerDiagnosticBuilder {
    pub fn new(source_code: impl Into<NamedSource>, error: impl Into<anyhow::Error>) -> Self {
        Self {
            source_code: source_code.into(),
            labels: None,
            help: None,
            error_source: error.into(),
            related_errors: None,
        }
    }

    pub fn label(mut self, label: LabeledSpan) -> Self {
        let mut labels = self.labels.unwrap_or_else(|| Vec::with_capacity(1));
        labels.push(label);
        self.labels = Some(labels);
        self
    }

    pub fn optional_label(self, label: Option<LabeledSpan>) -> Self {
        if let Some(label) = label {
            self.label(label)
        } else {
            self
        }
    }

    pub fn optional_related_error(self, related_error: Option<CompilerDiagnostic>) -> Self {
        if let Some(related) = related_error {
            self.related_error(related)
        } else {
            self
        }
    }

    pub fn related_error(mut self, related_error: CompilerDiagnostic) -> Self {
        let mut related_errors = self.related_errors.unwrap_or_else(|| Vec::with_capacity(1));
        related_errors.push(related_error);
        self.related_errors = Some(related_errors);
        self
    }

    pub fn help(mut self, help: String) -> Self {
        self.help = Some(help);
        self
    }

    pub fn build(self) -> CompilerDiagnostic {
        let Self {
            source_code,
            labels,
            help,
            error_source,
            related_errors,
        } = self;
        CompilerDiagnostic {
            source_code,
            labels,
            help,
            error_source,
            related_errors,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{error_source}")]
pub struct CompilerDiagnostic {
    source_code: NamedSource,
    labels: Option<Vec<LabeledSpan>>,
    help: Option<String>,
    #[source]
    error_source: anyhow::Error,
    related_errors: Option<Vec<CompilerDiagnostic>>,
}

impl miette::Diagnostic for CompilerDiagnostic {
    fn help<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        self.help
            .as_ref()
            .map(|s| Box::new(s) as Box<dyn Display + 'a>)
    }

    fn source_code(&self) -> Option<&dyn SourceCode> {
        Some(&self.source_code)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        self.labels
            .clone()
            .map(|l| Box::new(l.into_iter()) as Box<dyn Iterator<Item = LabeledSpan> + '_>)
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>> {
        self.related_errors.as_ref().map(|errors| {
            Box::new(errors.iter().map(|e| e as &dyn Diagnostic))
                as Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>
        })
    }
}

/// Location, obtained via `#[track_caller]` and `std::panic::Location::caller`, points at the
/// `.` in the method invocation for `route` and `constructor`.
/// E.g.
///
/// ```rust,ignore
/// App::builder()
///   .route(f!(crate::stream_file::<std::path::PathBuf>), "/home")
/// //^ `location` points here!
/// ```
///
/// We want build a `SourceSpan` that matches the `f!` invocation.
/// E.g.
///
/// ```rust,ignore
/// App::builder()
///   .route(f!(crate::stream_file::<std::path::PathBuf>), "/home")
/// //       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
/// //       We want a SourceSpan that points at this!
/// ```
///
/// How do we do it?
/// We parse the source file via `syn` and then visit the abstract syntax tree.
/// We know that we are looking for a method call, so we test every method call node to see
/// if `location` falls within its span.
/// We then convert the span associated with the node to a [`miette::SourceSpan`].
///
/// # Ambiguity
///
/// There are going to be multiple nodes that match if we are dealing with chained method calls.
/// Luckily enough, the visit is pre-order, therefore the latest node that contains `location`
/// is also the smallest node that contains it - exactly what we are looking for.
pub fn get_f_macro_invocation_span(
    raw_source: &str,
    parsed_source: &syn::File,
    location: &Location,
) -> Option<SourceSpan> {
    struct CallableLocator<'a> {
        location: &'a Location,
        node: Option<&'a ExprMethodCall>,
    }

    impl<'a> syn::visit::Visit<'a> for CallableLocator<'a> {
        fn visit_expr_method_call(&mut self, node: &'a ExprMethodCall) {
            if node.span().contains(self.location) {
                self.node = Some(node);
                syn::visit::visit_expr_method_call(self, node)
            }
        }

        fn visit_stmt(&mut self, node: &'a Stmt) {
            // This is an optimization - it allows the visitor to skip the entire sub-tree
            // under a top-level statement that is not relevant to our search.
            if node.span().contains(self.location) {
                syn::visit::visit_stmt(self, node)
            }
        }
    }

    let mut locator = CallableLocator {
        location,
        node: None,
    };
    locator.visit_file(parsed_source);
    if let Some(node) = locator.node {
        if let Some(argument) = node.args.first() {
            return Some(convert_span(raw_source, argument.span()));
        }
    }
    None
}

/// Helper methods to reduce boilerplate when working with [`miette::SourceSpan`]s.  
/// We might eventually want to upstream them.
pub trait SourceSpanExt {
    fn labeled(self, label_msg: String) -> miette::LabeledSpan;
    fn unlabeled(self) -> miette::LabeledSpan;
}

impl SourceSpanExt for miette::SourceSpan {
    fn labeled(self, label_msg: String) -> LabeledSpan {
        LabeledSpan::new_with_span(Some(label_msg), self)
    }

    fn unlabeled(self) -> LabeledSpan {
        LabeledSpan::new_with_span(None, self)
    }
}

pub trait ProcMacroSpanExt {
    fn contains(&self, location: &Location) -> bool;
}

impl ProcMacroSpanExt for proc_macro2::Span {
    fn contains(&self, location: &Location) -> bool {
        let span_start = self.start();
        if span_start.line < location.line as usize
            || (span_start.line == location.line as usize
                && span_start.column <= location.column as usize)
        {
            let span_end = self.end();
            if span_end.line > location.line as usize
                || (span_end.line == location.line as usize
                    && span_end.column >= location.column as usize)
            {
                return true;
            }
        }
        false
    }
}

pub fn convert_span(source: &str, span: proc_macro2::Span) -> miette::SourceSpan {
    // No idea why the +1 is required, but I kept getting labeled spans that were _consistently_
    // shifted by one character. So...
    let start = SourceOffset::from_location(source, span.start().line, span.start().column + 1);
    let end = SourceOffset::from_location(source, span.end().line, span.end().column + 1);
    let len = end.offset() - start.offset();
    SourceSpan::from((start.offset(), len))
}

pub fn convert_rustdoc_span(source: &str, span: rustdoc_types::Span) -> miette::SourceSpan {
    // No idea why the +1 is required, but I kept getting labeled spans that were _consistently_
    // shifted by one character. So...
    let start = SourceOffset::from_location(source, span.begin.0, span.begin.1 + 1);
    let end = SourceOffset::from_location(source, span.end.0, span.end.1 + 1);
    let len = end.offset() - start.offset();
    SourceSpan::from((start.offset(), len))
}

#[derive(Debug)]
pub(crate) struct ParsedSourceFile {
    pub(crate) path: std::path::PathBuf,
    pub(crate) contents: String,
    pub(crate) parsed: syn::File,
}

impl ParsedSourceFile {
    pub fn new(
        path: std::path::PathBuf,
        workspace: &guppy::graph::Workspace,
    ) -> Result<Self, std::io::Error> {
        let source = read_source_file(&path, workspace)?;
        let parsed = syn::parse_str(&source).unwrap();
        Ok(Self {
            path,
            contents: source,
            parsed,
        })
    }
}

impl From<ParsedSourceFile> for NamedSource {
    fn from(f: ParsedSourceFile) -> Self {
        let file_name = f.path.to_string_lossy();
        NamedSource::new(file_name, f.contents)
    }
}

/// Given a file path, return the content of the source file it refers to.
///
/// Relative paths are assumed to be relative to the workspace root manifest.  
/// Absolute paths are used as-is.
pub fn read_source_file(
    path: &Path,
    workspace: &guppy::graph::Workspace,
) -> Result<String, std::io::Error> {
    if path.is_absolute() {
        fs_err::read_to_string(path)
    } else {
        let path = workspace.root().as_std_path().join(&path);
        fs_err::read_to_string(&path)
    }
}
