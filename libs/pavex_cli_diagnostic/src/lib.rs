use std::fmt::Display;

use miette::{Diagnostic, LabeledSpan, NamedSource, Severity, SourceCode};

mod utils;

pub use utils::{AnyhowBridge, InteropError};

/// A builder for a [`CompilerDiagnostic`].
pub struct CompilerDiagnosticBuilder {
    severity: Severity,
    help: Option<String>,
    help_with_snippet: Option<Vec<HelpWithSnippet<NamedSource<String>>>>,
    error_source: anyhow::Error,
    annotated_sources: Vec<AnnotatedSource<NamedSource<String>>>,
}

impl CompilerDiagnosticBuilder {
    fn new(error: impl Into<anyhow::Error>) -> Self {
        Self {
            severity: Severity::Error,
            help: None,
            help_with_snippet: None,
            error_source: error.into(),
            annotated_sources: Vec::new(),
        }
    }

    /// Change the severity of this diagnostic.
    /// [`Severity::Error`] is the default.
    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// An optional version of [`Self::help`].
    pub fn optional_help(self, help: Option<String>) -> Self {
        if let Some(help) = help {
            self.help(help)
        } else {
            self
        }
    }

    /// An optional version of [`Self::source`].
    pub fn optional_source<S: Into<NamedSource<String>>>(
        self,
        annotated_snippet: Option<AnnotatedSource<S>>,
    ) -> Self {
        if let Some(s) = annotated_snippet {
            self.source(s)
        } else {
            self
        }
    }

    /// Attach an annotated source file to this diagnostic.
    pub fn source<S: Into<NamedSource<String>>>(
        self,
        annotated_snippet: AnnotatedSource<S>,
    ) -> Self {
        self.sources(std::iter::once(annotated_snippet.normalize()))
    }

    /// Attach additional annotated source files to be displayed with this diagnostic.
    pub fn sources<S: Into<NamedSource<String>>>(
        mut self,
        sources: impl Iterator<Item = AnnotatedSource<S>>,
    ) -> Self {
        self.annotated_sources
            .extend(sources.into_iter().map(|s| s.normalize()));
        self
    }

    /// An optional version of [`Self::help_with_snippet`].
    pub fn optional_help_with_snippet<S: Into<NamedSource<String>>>(
        self,
        help: Option<HelpWithSnippet<S>>,
    ) -> Self {
        if let Some(help) = help {
            self.help_with_snippet(help)
        } else {
            self
        }
    }

    /// Add an help message to this diagnostic to nudge the user in the right direction.
    /// The help message includes an annotated code snippet.
    pub fn help_with_snippet<S: Into<NamedSource<String>>>(
        mut self,
        help: HelpWithSnippet<S>,
    ) -> Self {
        let mut annotated_snippets = self
            .help_with_snippet
            .unwrap_or_else(|| Vec::with_capacity(1));
        annotated_snippets.push(help.normalize());
        self.help_with_snippet = Some(annotated_snippets);
        self
    }

    /// Add an help message to this diagnostic to nudge the user in the right direction.
    ///
    /// Help messages are rendered at the very end of the diagnostic, after the error message
    /// and all code snippets.
    pub fn help(mut self, help: String) -> Self {
        self.help = Some(help);
        self
    }

    /// Finalize the builder and return a [`CompilerDiagnostic`].
    pub fn build(self) -> CompilerDiagnostic {
        let Self {
            severity,
            help,
            help_with_snippet,
            error_source,
            mut annotated_sources,
        } = self;

        // miette doesn't have first-class support for attaching multiple annotated code snippets
        // to a single diagnostic _if_ they are not all related to the same source file.
        // We work around this by creating a separate related diagnostic for each annotated snippet,
        // with an empty error message.
        // Our custom `miette` handler in `pavex_miette` will make sure to render the "related errors"
        // as additional annotated snippets, which is what we want to see.

        let related = if annotated_sources.len() > 1 {
            annotated_sources.drain(1..).collect()
        } else {
            Vec::new()
        }
        .into_iter()
        .map(|s| SimpleDiagnostic {
            source_code: s.source_code,
            severity,
            labels: if s.labels.is_empty() {
                None
            } else {
                Some(s.labels)
            },
            help: None,
            error_source: anyhow::anyhow!(""),
        });

        let primary_source = if annotated_sources.is_empty() {
            AnnotatedSource::empty()
        } else {
            annotated_sources.remove(0)
        };
        let primary = SimpleDiagnostic {
            source_code: primary_source.source_code,
            severity,
            labels: Some(primary_source.labels),
            help,
            error_source,
        };

        CompilerDiagnostic {
            primary,
            related: Some(
                related
                    .chain(help_with_snippet.unwrap_or_default().into_iter().map(|s| {
                        SimpleDiagnostic {
                            source_code: s.snippet.source_code,
                            severity: Severity::Advice,
                            labels: Some(s.snippet.labels),
                            help: None,
                            error_source: anyhow::anyhow!("{}", s.help),
                        }
                    }))
                    .collect(),
            ),
        }
    }
}

impl CompilerDiagnostic {
    /// Start building a diagnostic.
    /// You must specify the error that caused the diagnostic.
    ///
    /// You can optionally specify:
    ///
    /// - the source code the diagnostic refer to (see [`CompilerDiagnosticBuilder::source`]
    ///   and [`CompilerDiagnosticBuilder::optional_source`])
    /// - labels to highlight specific parts of the source code (see
    ///   [`CompilerDiagnosticBuilder::label`] and [`CompilerDiagnosticBuilder::optional_label`]);
    /// - a help message to provide more information about the error (see
    ///   [`CompilerDiagnosticBuilder::help`] and [`CompilerDiagnosticBuilder::optional_help`]);
    /// - related errors. This can be leveraged to point at other source files that are related
    ///   to the error (see [`CompilerDiagnosticBuilder::additional_annotated_snippet`] and
    ///   [`CompilerDiagnosticBuilder::optional_additional_annotated_snippet`]).
    pub fn builder(error: impl Into<anyhow::Error>) -> CompilerDiagnosticBuilder {
        CompilerDiagnosticBuilder::new(error)
    }
}

/// An help message supported by an annotated code snippet.
pub struct HelpWithSnippet<S> {
    help: String,
    snippet: AnnotatedSource<S>,
}

impl<S> HelpWithSnippet<S>
where
    S: Into<NamedSource<String>>,
{
    pub fn new(help: String, snippet: AnnotatedSource<S>) -> Self {
        Self { help, snippet }
    }

    pub fn normalize(self) -> HelpWithSnippet<NamedSource<String>> {
        HelpWithSnippet {
            help: self.help,
            snippet: self.snippet.normalize(),
        }
    }
}

/// A source file annotated with one or more labels.
pub struct AnnotatedSource<S> {
    pub source_code: S,
    pub labels: Vec<LabeledSpan>,
}

impl AnnotatedSource<NamedSource<String>> {
    /// Create an empty annotated source—no source and no labels.
    pub fn empty() -> Self {
        Self::new(NamedSource::new(String::new(), String::new()))
    }
}

impl<S> AnnotatedSource<S>
where
    S: Into<NamedSource<String>>,
{
    /// Build a new annotated source with a single label.
    pub fn new(source_code: S) -> Self {
        Self {
            source_code,
            labels: Vec::new(),
        }
    }

    /// Convert the underlying source type into a [`NamedSource<String>`].
    pub fn normalize(self) -> AnnotatedSource<NamedSource<String>> {
        AnnotatedSource {
            source_code: self.source_code.into(),
            labels: self.labels,
        }
    }
}

impl<S> AnnotatedSource<S> {
    /// A reference to the source code.
    pub fn source(&self) -> &S {
        &self.source_code
    }

    /// Add an extra label to this annotated source.
    pub fn label<L: Into<Option<LabeledSpan>>>(mut self, label: L) -> Self {
        if let Some(label) = label.into() {
            self.labels.push(label);
        }
        self
    }

    /// Add multiple labels to this annotated source.
    pub fn labels(mut self, labels: Vec<LabeledSpan>) -> Self {
        self.labels.extend(labels);
        self
    }
}

#[derive(Debug)]
/// A diagnostic is a single error or warning message returned by Pavex to the user.
///
/// See [`CompilerDiagnostic::builder`] for how to create a diagnostic.
pub struct CompilerDiagnostic {
    primary: SimpleDiagnostic,
    related: Option<Vec<SimpleDiagnostic>>,
}

impl std::fmt::Display for CompilerDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.primary)
    }
}

impl std::error::Error for CompilerDiagnostic {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.primary.source()
    }
}

impl miette::Diagnostic for CompilerDiagnostic {
    fn severity(&self) -> Option<Severity> {
        Some(self.primary.severity)
    }

    fn help<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        self.primary
            .help
            .as_ref()
            .map(|s| Box::new(s) as Box<dyn Display + 'a>)
    }

    fn source_code(&self) -> Option<&dyn SourceCode> {
        Some(&self.primary.source_code)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        self.primary
            .labels
            .clone()
            .map(|l| Box::new(l.into_iter()) as Box<dyn Iterator<Item = LabeledSpan> + '_>)
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>> {
        self.related.as_ref().map(|errors| {
            Box::new(errors.iter().map(|e| e as &dyn Diagnostic))
                as Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>
        })
    }
}

#[derive(Debug)]
/// A simple diagnostic coalesces together multiple labels for the same source file.
struct SimpleDiagnostic {
    source_code: NamedSource<String>,
    severity: Severity,
    labels: Option<Vec<LabeledSpan>>,
    help: Option<String>,
    error_source: anyhow::Error,
}

impl std::fmt::Display for SimpleDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error_source)
    }
}

impl std::error::Error for SimpleDiagnostic {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // Return the source of the source error, if one exists
        self.error_source.source()
    }
}

impl miette::Diagnostic for SimpleDiagnostic {
    fn severity(&self) -> Option<Severity> {
        Some(self.severity)
    }

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
        None
    }
}
