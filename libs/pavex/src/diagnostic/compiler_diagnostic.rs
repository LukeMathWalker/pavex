use std::fmt::Display;

use miette::{Diagnostic, LabeledSpan, NamedSource, SourceCode};

/// A builder for a [`CompilerDiagnostic`].
pub struct CompilerDiagnosticBuilder {
    source_code: NamedSource,
    labels: Option<Vec<LabeledSpan>>,
    help: Option<String>,
    error_source: anyhow::Error,
    related_errors: Option<Vec<CompilerDiagnostic>>,
}

impl CompilerDiagnosticBuilder {
    fn new(source_code: impl Into<NamedSource>, error: impl Into<anyhow::Error>) -> Self {
        Self {
            source_code: source_code.into(),
            labels: None,
            help: None,
            error_source: error.into(),
            related_errors: None,
        }
    }

    /// Overwrite the source error for this diagnostic.
    pub fn error(mut self, error: impl Into<anyhow::Error>) -> Self {
        self.error_source = error.into();
        self
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

    pub fn optional_help(self, help: Option<String>) -> Self {
        if let Some(help) = help {
            self.help(help)
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

    /// Finalize the builder and return a [`CompilerDiagnostic`].
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

impl CompilerDiagnostic {
    /// Start building a diagnostic.
    /// You must specify:
    ///
    /// - the source code that the diagnostic is about;
    /// - the error that caused the diagnostic.
    ///
    /// You can optionally specify:
    ///
    /// - labels to highlight specific parts of the source code (see
    /// [`CompilerDiagnosticBuilder::label`] and [`CompilerDiagnosticBuilder::optional_label`]);
    /// - a help message to provide more information about the error (see
    /// [`CompilerDiagnosticBuilder::help`] and [`CompilerDiagnosticBuilder::optional_help`]);
    /// - related errors. This can be leveraged to point at other source files that are related
    /// to the error (see [`CompilerDiagnosticBuilder::related_error`] and
    /// [`CompilerDiagnosticBuilder::optional_related_error`]).
    pub fn builder(
        source_code: impl Into<NamedSource>,
        error: impl Into<anyhow::Error>,
    ) -> CompilerDiagnosticBuilder {
        CompilerDiagnosticBuilder::new(source_code, error)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{error_source}")]
/// A diagnostic is a single error or warning message returned by `pavex` to the user.
///
/// See [`CompilerDiagnostic::builder`] for how to create a diagnostic.
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
