use crate::{
    diagnostic::{ComponentKind, DiagnosticSink, Registration, TargetSpan},
    rustdoc::RustdocKindExt,
};
use pavex_cli_diagnostic::{AnnotatedSource, CompilerDiagnostic, HelpWithSnippet};
use pavexc_attr_parser::{AnnotationKind, errors::AttributeParserError};
use rustdoc_types::Item;

pub(crate) fn invalid_diagnostic_attribute(
    e: AttributeParserError,
    item: &Item,
    diagnostics: &DiagnosticSink,
) {
    let source = item.span.as_ref().and_then(|s| {
        diagnostics.annotated(
            TargetSpan::Registration(&Registration::attribute(s), ComponentKind::Constructor),
            "The annotated item",
        )
    });
    let err_msg = match &item.name {
        Some(name) => {
            format!("`{name}` is annotated with a malformed `diagnostic::pavex::*` attribute.",)
        }
        None => "One of your items is annotated with a malformed `diagnostic::pavex::*` attribute."
            .into(),
    };
    let diagnostic = CompilerDiagnostic::builder(anyhow::anyhow!(e.to_string()).context(err_msg))
        .optional_source(source)
        .help("Have you manually added the `diagnostic::pavex::*` attribute on the item? \
            The syntax for `diagnostic::pavex::*` attributes is an implementation detail of Pavex's own macros, \
            which are guaranteed to output well-formed annotations.".into())
        .build();
    diagnostics.push(diagnostic);
}

pub(super) fn missing_methods_attribute(
    kind: AnnotationKind,
    impl_item: &Item,
    item: &Item,
    diagnostics: &DiagnosticSink,
) {
    let source = item.span.as_ref().and_then(|s| {
        diagnostics.annotated(
            TargetSpan::Registration(&Registration::attribute(s), ComponentKind::Constructor),
            format!("The {kind}"),
        )
    });
    let err_msg = match &item.name {
        Some(name) => {
            format!(
                "Missing `#[pavex::methods]` attribute on the `impl` block that defines `{name}`, a Pavex {kind}.",
            )
        }
        None => {
            format!(
                "Missing `#[pavex::methods]` attribute on an `impl` block that defines a Pavex {kind}."
            )
        }
    };

    let help_annotation = impl_item.span.as_ref().and_then(|s| {
        diagnostics.annotated(
            TargetSpan::Impl(&Registration::attribute(s)),
            "Add #[pavex::methods] right above this line",
        )
    });
    let help_msg = "Add `#[pavex::methods]` as an attribute on top of the `impl` block.";
    let help = match help_annotation {
        Some(a) => HelpWithSnippet::new(help_msg.into(), a).normalize(),
        None => HelpWithSnippet::new(help_msg.into(), AnnotatedSource::empty()),
    };

    let diagnostic = CompilerDiagnostic::builder(anyhow::anyhow!(err_msg))
        .optional_source(source)
        .help_with_snippet(help)
        .build();
    diagnostics.push(diagnostic);
}

pub(super) fn unsupported_item_kind(attribute: &str, item: &Item, diagnostics: &DiagnosticSink) {
    let source = item.span.as_ref().and_then(|s| {
        diagnostics.annotated(
            TargetSpan::Registration(&Registration::attribute(s), ComponentKind::Constructor),
            "The annotated item",
        )
    });
    let err = match &item.name {
        Some(name) => {
            format!(
                "`{name}` is annotated with `{attribute}`, but `{attribute}` is not supported on {}.",
                item.inner.kind()
            )
        }
        None => format!("`{attribute}` is not supported on {}.", item.inner.kind()),
    };
    let diagnostic = CompilerDiagnostic::builder(anyhow::anyhow!(err))
        .optional_source(source)
        .help(format!("Have you manually added `{attribute}`? \
            The syntax for `diagnostic::pavex::*` attributes is an implementation detail of Pavex's own macros, \
            which are guaranteed to output well-formed annotations."))
        .build();
    diagnostics.push(diagnostic);
}
