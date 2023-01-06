use miette::{LabeledSpan, SourceOffset, SourceSpan};

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
