use std::fmt::Display;

use miette::{Diagnostic, LabeledSpan, NamedSource, Severity, SourceCode};

/// A builder for a [`CompilerDiagnostic`].
pub struct CompilerDiagnosticBuilder {
    severity: Severity,
    source_code: Option<NamedSource<String>>,
    labels: Option<Vec<LabeledSpan>>,
    help: Option<String>,
    help_with_snippet: Option<Vec<HelpWithSnippet>>,
    error_source: anyhow::Error,
    additional_annotated_snippets: Option<Vec<AnnotatedSnippet>>,
}

impl CompilerDiagnosticBuilder {
    fn new(error: impl Into<anyhow::Error>) -> Self {
        Self {
            severity: Severity::Error,
            source_code: None,
            labels: None,
            help: None,
            help_with_snippet: None,
            error_source: error.into(),
            additional_annotated_snippets: None,
        }
    }

    /// Attach a source file to this diagnostic.
    pub fn source(mut self, source: impl Into<NamedSource<String>>) -> Self {
        self.source_code = Some(source.into());
        self
    }

    /// An optional version of [`CompilerDiagnosticBuilder::source`].
    pub fn optional_source(mut self, source: Option<impl Into<NamedSource<String>>>) -> Self {
        self.source_code = source.map(Into::into);
        self
    }

    /// Change the severity of this diagnostic.
    /// [`Severity::Error`] is the default.
    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Add one more label to this diagnostic.
    /// If there are already labels, the new label will be appended.
    pub fn label(self, label: LabeledSpan) -> Self {
        self.labels(std::iter::once(label))
    }

    /// Add multiple labels to this diagnostic.
    /// If there are already labels, the new labels will be appended.
    pub fn labels(mut self, new_labels: impl Iterator<Item = LabeledSpan>) -> Self {
        let mut labels = self.labels.unwrap_or_else(|| Vec::with_capacity(1));
        labels.extend(new_labels);
        self.labels = Some(labels);
        self
    }

    /// An optional version of [`Self::label`].
    pub fn optional_label(self, label: Option<LabeledSpan>) -> Self {
        if let Some(label) = label {
            self.label(label)
        } else {
            self
        }
    }

    /// An optional version of [`Self::help`].
    pub fn optional_help(self, help: Option<String>) -> Self {
        if let Some(help) = help {
            self.help(help)
        } else {
            self
        }
    }

    /// An optional version of [`Self::additional_annotated_snippet`].
    pub fn optional_additional_annotated_snippet(
        self,
        annotated_snippet: Option<AnnotatedSnippet>,
    ) -> Self {
        if let Some(s) = annotated_snippet {
            self.additional_annotated_snippet(s)
        } else {
            self
        }
    }

    /// Record an additional annotated code snippet to be displayed with this diagnostic.
    ///
    /// This can be used to display a code snippet that doesn't live in the same source file
    /// as the main code snippet for this diagnostic.
    pub fn additional_annotated_snippet(self, annotated_snippet: AnnotatedSnippet) -> Self {
        self.additional_annotated_snippets(std::iter::once(annotated_snippet))
    }

    /// Record an additional annotated code snippet to be displayed with this diagnostic.
    ///
    /// This can be used to display a code snippet that doesn't live in the same source file
    /// as the main code snippet for this diagnostic.
    pub fn additional_annotated_snippets(
        mut self,
        new_snippets: impl Iterator<Item = AnnotatedSnippet>,
    ) -> Self {
        let mut annotated_snippets = self
            .additional_annotated_snippets
            .unwrap_or_else(|| Vec::with_capacity(1));
        annotated_snippets.extend(new_snippets);
        self.additional_annotated_snippets = Some(annotated_snippets);
        self
    }

    /// An optional version of [`Self::help_with_snippet`].
    pub fn optional_help_with_snippet(self, help: Option<HelpWithSnippet>) -> Self {
        if let Some(help) = help {
            self.help_with_snippet(help)
        } else {
            self
        }
    }

    /// Add an help message to this diagnostic to nudge the user in the right direction.
    /// The help message includes an annotated code snippet.
    pub fn help_with_snippet(mut self, help: HelpWithSnippet) -> Self {
        let mut annotated_snippets = self
            .help_with_snippet
            .unwrap_or_else(|| Vec::with_capacity(1));
        annotated_snippets.push(help);
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
            source_code,
            labels,
            help,
            help_with_snippet,
            error_source,
            additional_annotated_snippets,
        } = self;
        // miette doesn't have first-class support for attaching multiple annotated code snippets
        // to a single diagnostic _if_ they are not all related to the same source file.
        // We work around this by creating a separate related diagnostic for each annotated snippet,
        // with an empty error message.
        // Our custom `miette` handler in `pavex_miette` will make sure to render the "related errors"
        // as additional annotated snippets, which is what we want to see.
        let mut related_diagnostics = additional_annotated_snippets
            .map(|snippets| {
                snippets
                    .into_iter()
                    .map(|s| {
                        CompilerDiagnosticBuilder::new(anyhow::anyhow!(""))
                            .source(s.source_code)
                            .labels(s.labels.into_iter())
                            .build()
                    })
                    .collect()
            })
            .unwrap_or(Vec::new());

        if let Some(helps) = help_with_snippet {
            related_diagnostics.extend(helps.into_iter().map(|s| {
                let mut d = CompilerDiagnosticBuilder::new(anyhow::anyhow!("{}", s.help))
                    .source(s.snippet.source_code)
                    .labels(s.snippet.labels.into_iter())
                    .build();
                d.severity = Severity::Advice;
                d
            }))
        }
        CompilerDiagnostic {
            source_code: source_code
                .unwrap_or_else(|| NamedSource::new(String::new(), String::new())),
            severity,
            labels,
            help,
            error_source,
            related_errors: Some(related_diagnostics),
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
pub struct HelpWithSnippet {
    help: String,
    snippet: AnnotatedSnippet,
}

impl HelpWithSnippet {
    pub fn new(help: String, snippet: AnnotatedSnippet) -> Self {
        Self { help, snippet }
    }
}

/// A source file annotated with one or more labels.
pub struct AnnotatedSnippet {
    pub source_code: NamedSource<String>,
    pub labels: Vec<LabeledSpan>,
}

impl AnnotatedSnippet {
    /// Build a new annotated snippet with a single label.
    pub fn new(source_code: impl Into<NamedSource<String>>, label: LabeledSpan) -> Self {
        Self {
            source_code: source_code.into(),
            labels: vec![label],
        }
    }

    /// Create an empty annotated snippet—no source and no labels.
    pub fn empty() -> Self {
        Self {
            source_code: NamedSource::new(String::new(), String::new()),
            labels: Vec::new(),
        }
    }

    /// Build a new annotated snippet with an optional single label.
    pub fn new_optional(
        source_code: impl Into<NamedSource<String>>,
        label: Option<LabeledSpan>,
    ) -> Self {
        Self {
            source_code: source_code.into(),
            labels: label.map(|l| vec![l]).unwrap_or_default(),
        }
    }

    /// Build a new annotated snippet with multiple labels.
    pub fn new_with_labels(
        source_code: impl Into<NamedSource<String>>,
        labels: Vec<LabeledSpan>,
    ) -> Self {
        Self {
            source_code: source_code.into(),
            labels,
        }
    }

    /// Add an extra label to this annotated snippet.
    #[allow(unused)]
    pub fn add_label(mut self, label: LabeledSpan) -> Self {
        self.labels.push(label);
        self
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{error_source}")]
/// A diagnostic is a single error or warning message returned by Pavex to the user.
///
/// See [`CompilerDiagnostic::builder`] for how to create a diagnostic.
pub struct CompilerDiagnostic {
    source_code: NamedSource<String>,
    severity: Severity,
    labels: Option<Vec<LabeledSpan>>,
    help: Option<String>,
    #[source]
    error_source: anyhow::Error,
    related_errors: Option<Vec<CompilerDiagnostic>>,
}

impl miette::Diagnostic for CompilerDiagnostic {
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
        self.related_errors.as_ref().map(|errors| {
            Box::new(errors.iter().map(|e| e as &dyn Diagnostic))
                as Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>
        })
    }
}
