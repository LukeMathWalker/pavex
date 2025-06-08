use crate::{
    compiler::analyses::user_components::imports::UnresolvedImport,
    diagnostic::{
        self, ComponentKind, DiagnosticSink, OptionalLabeledSpanExt, OptionalSourceSpanExt,
        Registration, TargetSpan,
    },
};
use pavex_cli_diagnostic::CompilerDiagnostic;
use rustdoc_types::Item;

use super::ConstGenericsAreNotSupported;

pub(super) fn const_generics_are_not_supported(
    e: ConstGenericsAreNotSupported,
    item: &Item,
    diagnostics: &DiagnosticSink,
) {
    let source = item.span.as_ref().and_then(|s| {
        diagnostics.annotated(
            TargetSpan::Registration(&Registration::attribute(s), ComponentKind::Constructor),
            "The annotated item",
        )
    });
    let const_name = e.name;
    let err_msg = match &item.name {
        Some(name) => {
            format!(
                "Pavex does not support const generics.\n`{name}` uses at least one const generic parameter, named `{const_name}`.",
            )
        }
        None => format!(
            "Pavex does not support const generics.\nOne of your types uses at least one const generic parameter, named `{const_name}`."
        ),
    };
    let diagnostic = CompilerDiagnostic::builder(anyhow::anyhow!(err_msg))
        .optional_source(source)
        .help("Remove the const generic parameter from your type definition, or use a different type.".into())
        .build();
    diagnostics.push(diagnostic);
}

pub(super) fn unknown_module_path(
    module_path: &[String],
    krate_name: &str,
    import: &UnresolvedImport,
    diagnostics: &DiagnosticSink,
) {
    let source = diagnostics.source(&import.registered_at).map(|s| {
        diagnostic::imported_sources_span(s.source(), &import.registered_at)
            .labeled("The import was registered here".into())
            .attach(s)
    });
    let module_path = module_path.join("::");
    let e = anyhow::anyhow!(
        "You tried to import from `{module_path}`, but there is no module with that path in `{krate_name}`."
    );
    let diagnostic = CompilerDiagnostic::builder(e)
        .optional_source(source)
        .build();
    diagnostics.push(diagnostic);
}

pub(super) fn not_a_module(
    path: &[String],
    import: &UnresolvedImport,
    diagnostics: &DiagnosticSink,
) {
    let source = diagnostics.source(&import.registered_at).map(|s| {
        diagnostic::imported_sources_span(s.source(), &import.registered_at)
            .labeled("The import was registered here".into())
            .attach(s)
    });
    let path = path.join("::");
    let e = anyhow::anyhow!("You tried to import from `{path}`, but `{path}` is not a module.");
    let diagnostic = CompilerDiagnostic::builder(e)
        .optional_source(source)
        .help(
            "Specify the path of the module that contains the item you want to import, rather than the actual item path.".into()
        )
        .build();
    diagnostics.push(diagnostic);
}
