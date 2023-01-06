// pub use compiler_diagnostic::{CompilerDiagnostic, CompilerDiagnosticBuilder};
use std::path::Path;

use guppy::graph::PackageGraph;
use miette::{LabeledSpan, MietteError, NamedSource, SourceOffset, SourceSpan};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{ExprMethodCall, Stmt};

pub use compiler_diagnostic::CompilerDiagnostic;
use pavex_builder::{AppBlueprint, Location, RawCallableIdentifiers};

mod compiler_diagnostic;

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
/// //       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
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
pub(crate) fn get_f_macro_invocation_span(
    source: &ParsedSourceFile,
    location: &Location,
) -> Option<SourceSpan> {
    struct CallableLocator<'a> {
        location: &'a Location,
        node: Option<&'a ExprMethodCall>,
    }

    impl<'a> Visit<'a> for CallableLocator<'a> {
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

    let raw_source = &source.contents;
    let parsed_source = &source.parsed;
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
    fn labeled(self, label_msg: String) -> LabeledSpan;
    fn unlabeled(self) -> LabeledSpan;
}

/// Helper methods to reduce boilerplate when working with an optional [`miette::SourceSpan`].  
pub trait OptionalSourceSpanExt {
    fn labeled(self, label_msg: String) -> Option<LabeledSpan>;
    fn unlabeled(self) -> Option<LabeledSpan>;
}

impl OptionalSourceSpanExt for Option<SourceSpan> {
    fn labeled(self, label_msg: String) -> Option<LabeledSpan> {
        self.map(|s| s.labeled(label_msg))
    }

    fn unlabeled(self) -> Option<LabeledSpan> {
        self.map(|s| s.unlabeled())
    }
}

impl SourceSpanExt for SourceSpan {
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

pub fn convert_span(source: &str, span: proc_macro2::Span) -> SourceSpan {
    // No idea why the +1 is required, but I kept getting labeled spans that were _consistently_
    // shifted by one character. So...
    let start = SourceOffset::from_location(source, span.start().line, span.start().column + 1);
    let end = SourceOffset::from_location(source, span.end().line, span.end().column + 1);
    let len = end.offset() - start.offset();
    SourceSpan::from((start.offset(), len))
}

pub fn convert_rustdoc_span(source: &str, span: rustdoc_types::Span) -> SourceSpan {
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

pub(crate) trait LocationExt {
    fn source_file(&self, package_graph: &PackageGraph) -> Result<ParsedSourceFile, MietteError>;
}

impl LocationExt for Location {
    fn source_file(&self, package_graph: &PackageGraph) -> Result<ParsedSourceFile, MietteError> {
        ParsedSourceFile::new(self.file.as_str().into(), &package_graph.workspace())
            .map_err(MietteError::IoError)
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
        let path = workspace.root().as_std_path().join(path);
        fs_err::read_to_string(&path)
    }
}

/// Given a callable identifier, return the location where it was registered.
///
/// The same request handlers can be registered multiple times: this function returns the location
/// of the first registration.
pub(crate) fn get_registration_location<'a>(
    bp: &'a AppBlueprint,
    identifiers: &RawCallableIdentifiers,
) -> Option<&'a Location> {
    bp.constructor_locations
        .get(identifiers)
        .or_else(|| get_request_handler_location(bp, identifiers))
        .or_else(|| bp.error_handler_locations.get(identifiers))
}

/// Given the callable identifiers for a request handler, return the location where it was registered.
///
/// The same request handlers can be registered multiple times: this function returns the location
/// of the first registration.
pub(crate) fn get_request_handler_location<'a>(
    bp: &'a AppBlueprint,
    identifiers: &RawCallableIdentifiers,
) -> Option<&'a Location> {
    bp.request_handler_locations
        .get(identifiers)
        .and_then(|v| v.first())
}
