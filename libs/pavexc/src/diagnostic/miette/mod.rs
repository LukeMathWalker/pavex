use miette::{LabeledSpan, SourceOffset, SourceSpan};
use pavex_cli_diagnostic::AnnotatedSource;

/// Helper methods to reduce boilerplate when working with [`miette::SourceSpan`]s.
/// We might eventually want to upstream them.
pub trait SourceSpanExt {
    fn labeled(self, label_msg: String) -> LabeledSpan;
    fn unlabeled(self) -> LabeledSpan;
    fn shift(self, offset: usize) -> SourceSpan;
}

/// Helper methods to reduce boilerplate when working with an optional [`miette::SourceSpan`].
pub trait OptionalSourceSpanExt {
    fn labeled(self, label_msg: String) -> Option<LabeledSpan>;
    #[allow(unused)]
    fn unlabeled(self) -> Option<LabeledSpan>;
    #[allow(unused)]
    fn shift(self, offset: usize) -> Option<SourceSpan>;
}

impl OptionalSourceSpanExt for Option<SourceSpan> {
    fn labeled(self, label_msg: String) -> Option<LabeledSpan> {
        self.map(|s| s.labeled(label_msg))
    }

    fn unlabeled(self) -> Option<LabeledSpan> {
        self.map(|s| s.unlabeled())
    }

    fn shift(self, offset: usize) -> Option<SourceSpan> {
        self.map(|s| s.shift(offset))
    }
}

/// Helper methods to reduce boilerplate when working with a [`miette::LabeledSpan`]
/// and the respective source.
pub trait LabeledSpanExt {
    /// Add this labeled span to the given source.
    fn attach<S>(self, s: AnnotatedSource<S>) -> AnnotatedSource<S>;
}

impl LabeledSpanExt for LabeledSpan {
    #[must_use]
    fn attach<S>(self, s: AnnotatedSource<S>) -> AnnotatedSource<S> {
        s.label(self)
    }
}

/// Helper methods to reduce boilerplate when working with an optional [`miette::LabeledSpan`]
/// and the respective source.
pub trait OptionalLabeledSpanExt {
    /// Add this labeled span to the given source.
    fn attach<S>(self, s: AnnotatedSource<S>) -> AnnotatedSource<S>;
}

impl OptionalLabeledSpanExt for Option<LabeledSpan> {
    #[must_use]
    fn attach<S>(self, s: AnnotatedSource<S>) -> AnnotatedSource<S> {
        s.label(self)
    }
}

impl SourceSpanExt for SourceSpan {
    fn labeled(self, label_msg: String) -> LabeledSpan {
        LabeledSpan::new_with_span(Some(label_msg), self)
    }

    fn unlabeled(self) -> LabeledSpan {
        LabeledSpan::new_with_span(None, self)
    }

    fn shift(self, offset: usize) -> SourceSpan {
        SourceSpan::new((offset + self.offset()).into(), self.len())
    }
}

/// Convert a `proc_macro2::Span` to a `miette::SourceSpan`.
pub fn convert_proc_macro_span(source: &str, span: proc_macro2::Span) -> SourceSpan {
    // No idea why the +1 is required, but I kept getting labeled spans that were _consistently_
    // shifted by one character. So...
    let start = SourceOffset::from_location(source, span.start().line, span.start().column + 1);
    let end = SourceOffset::from_location(source, span.end().line, span.end().column + 1);
    let len = end.offset() - start.offset();
    SourceSpan::from((start.offset(), len))
}

/// Convert a `rustc_span::Span` to a `miette::SourceSpan`.
pub fn convert_rustdoc_span(source: &str, span: rustdoc_types::Span) -> SourceSpan {
    // No idea why the +1 is required, but I kept getting labeled spans that were _consistently_
    // shifted by one character. So...
    let start = SourceOffset::from_location(source, span.begin.0, span.begin.1 + 1);
    let end = SourceOffset::from_location(source, span.end.0, span.end.1 + 1);
    let len = end.offset() - start.offset();
    SourceSpan::from((start.offset(), len))
}
