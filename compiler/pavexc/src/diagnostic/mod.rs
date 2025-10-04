//! A toolkit to assemble and report errors and warnings to the user.
pub(crate) use self::miette::{
    LabeledSpanExt, OptionalLabeledSpanExt, OptionalSourceSpanExt, SourceSpanExt,
    convert_proc_macro_span, convert_rustdoc_span,
};
pub(crate) use callable_definition::CallableDefSource;
pub(crate) use kind::ComponentKind;
pub(crate) use ordinals::ZeroBasedOrdinal;
pub(crate) use pavex_cli_diagnostic::{
    AnnotatedSource, CompilerDiagnostic, CompilerDiagnosticBuilder, HelpWithSnippet,
};
pub(crate) use proc_macro_utils::ProcMacroSpanExt;
pub(crate) use registration::{Registration, RegistrationKind};
pub(crate) use registration_locations::{
    bp_new_span, config_key_span, domain_span, f_macro_span, imported_sources_span,
    nest_blueprint_span, prefix_span, registration_span, route_path_span,
};
pub use sink::DiagnosticSink;
pub(crate) use sink::TargetSpan;
pub(crate) use source_file::{LocationExt, ParsedSourceFile, read_source_file};

mod callable_definition;
mod kind;
mod miette;
mod ordinals;
mod proc_macro_utils;
mod registration;
mod registration_locations;
mod sink;
mod source_file;
