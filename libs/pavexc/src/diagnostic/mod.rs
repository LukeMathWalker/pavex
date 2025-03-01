//! A toolkit to assemble and report errors and warnings to the user.
use std::fmt::{Display, Formatter};

pub(crate) use ordinals::ZeroBasedOrdinal;
pub(crate) use pavex_cli_diagnostic::{
    AnnotatedSnippet, CompilerDiagnostic, CompilerDiagnosticBuilder, HelpWithSnippet,
};
pub(crate) use proc_macro_utils::ProcMacroSpanExt;
pub(crate) use registration_locations::{
    get_bp_new_span, get_config_key_span, get_domain_span, get_f_macro_invocation_span,
    get_nest_blueprint_span, get_prefix_span, get_route_path_span,
};
pub(crate) use source_file::{LocationExt, ParsedSourceFile, read_source_file};

pub(crate) use self::miette::{
    OptionalSourceSpanExt, SourceSpanExt, convert_proc_macro_span, convert_rustdoc_span,
};
pub(crate) use callable_definition::CallableDefinition;

mod callable_definition;
mod miette;
mod ordinals;
mod proc_macro_utils;
mod registration_locations;
mod source_file;
mod utils;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ComponentKind {
    RequestHandler,
    Constructor,
    ErrorHandler,
    WrappingMiddleware,
    PostProcessingMiddleware,
    PreProcessingMiddleware,
    ErrorObserver,
    PrebuiltType,
    ConfigType,
}

impl Display for ComponentKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ComponentKind::RequestHandler => "request handler",
            ComponentKind::Constructor => "constructor",
            ComponentKind::ErrorHandler => "error handler",
            ComponentKind::WrappingMiddleware => "wrapping middleware",
            ComponentKind::PostProcessingMiddleware => "post-processing middleware",
            ComponentKind::PreProcessingMiddleware => "pre-processing middleware",
            ComponentKind::ErrorObserver => "error observer",
            ComponentKind::PrebuiltType => "prebuilt type",
            ComponentKind::ConfigType => "config type",
        };
        write!(f, "{s}")
    }
}
