use crate::{
    diagnostic::{ComponentKind, DiagnosticSink, Registration, TargetSpan},
    rustdoc::Crate,
};
use pavex_cli_diagnostic::{AnnotatedSource, CompilerDiagnostic, HelpWithSnippet};
use rustdoc_ext::ItemKindExt;
use pavexc_annotations::IdConflict;
use pavexc_attr_parser::{AnnotationKind, errors::AttributeParserError};
use rustdoc_types::{ItemKind, Span};

pub(super) fn invalid_diagnostic_attribute(
    e: AttributeParserError,
    item_name: Option<&str>,
    item_span: Option<&Span>,
    diagnostics: &DiagnosticSink,
) {
    let source = item_span.and_then(|s| {
        diagnostics.annotated(
            TargetSpan::Registration(&Registration::attribute(s), ComponentKind::Constructor),
            "The annotated item",
        )
    });
    let err_msg = match item_name {
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
    item_name: Option<&str>,
    item_span: Option<&Span>,
    impl_span: Option<&Span>,
    diagnostics: &DiagnosticSink,
) {
    let source = item_span.and_then(|s| {
        diagnostics.annotated(
            TargetSpan::Registration(&Registration::attribute(s), ComponentKind::Constructor),
            format!("The {kind}"),
        )
    });
    let err_msg = match item_name {
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

    let help_annotation = impl_span.and_then(|s| {
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

pub(super) fn unsupported_item_kind(
    attribute: &str,
    item_name: Option<&str>,
    item_kind: ItemKind,
    item_span: Option<&Span>,
    diagnostics: &DiagnosticSink,
) {
    let source = item_span.and_then(|s| {
        diagnostics.annotated(
            TargetSpan::Registration(&Registration::attribute(s), ComponentKind::Constructor),
            "The annotated item",
        )
    });
    let kind_str = item_kind.plural();
    let err = match item_name {
        Some(name) => {
            format!(
                "`{name}` is annotated with `{attribute}`, but `{attribute}` is not supported on {kind_str}.",
            )
        }
        None => format!("`{attribute}` is not supported on {kind_str}."),
    };
    let diagnostic = CompilerDiagnostic::builder(anyhow::anyhow!(err))
        .optional_source(source)
        .help(format!("Have you manually added `{attribute}`? \
            The syntax for `diagnostic::pavex::*` attributes is an implementation detail of Pavex's own macros, \
            which are guaranteed to output well-formed annotations."))
        .build();
    diagnostics.push(diagnostic);
}

pub(super) fn id_conflict(
    e: IdConflict,
    first_span: Option<&Span>,
    second_span: Option<&Span>,
    _krate: &Crate,
    diagnostics: &DiagnosticSink,
) {
    let first_source = first_span.and_then(|s| {
        diagnostics.annotated(
            TargetSpan::Registration(&Registration::attribute(s), ComponentKind::Constructor),
            "The first component",
        )
    });
    let second_source = second_span.and_then(|s| {
        diagnostics.annotated(
            TargetSpan::Registration(&Registration::attribute(s), ComponentKind::Constructor),
            "The second component",
        )
    });
    let err = anyhow::anyhow!(
        "The identifier for Pavex components must be unique within the package where they are defined.\n\
        {} is used by two different components.",
        e.annotation_id
    );
    let diagnostic = CompilerDiagnostic::builder(anyhow::anyhow!(err))
        .optional_source(first_source)
        .optional_source(second_source)
        .help(
            "Use the `id` macro argument to change the identifier of one of the two components."
                .to_string(),
        )
        .build();
    diagnostics.push(diagnostic);
}
