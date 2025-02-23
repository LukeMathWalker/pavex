//! A toolkit to assemble and report errors and warnings to the user.
use std::fmt::{Display, Formatter};

pub(crate) use ordinals::ZeroBasedOrdinal;
pub(crate) use pavex_cli_diagnostic::{
    AnnotatedSnippet, CompilerDiagnostic, CompilerDiagnosticBuilder, HelpWithSnippet,
};
pub(crate) use proc_macro_utils::ProcMacroSpanExt;
pub(crate) use registration_locations::{
    get_bp_new_span, get_domain_span, get_f_macro_invocation_span, get_nest_blueprint_span,
    get_prefix_span, get_route_path_span,
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
pub enum CallableType {
    RequestHandler,
    Constructor,
    ErrorHandler,
    WrappingMiddleware,
    PostProcessingMiddleware,
    PreProcessingMiddleware,
    ErrorObserver,
    PrebuiltType,
}

impl Display for CallableType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CallableType::RequestHandler => "request handler",
            CallableType::Constructor => "constructor",
            CallableType::ErrorHandler => "error handler",
            CallableType::WrappingMiddleware => "wrapping middleware",
            CallableType::PostProcessingMiddleware => "post-processing middleware",
            CallableType::PreProcessingMiddleware => "pre-processing middleware",
            CallableType::ErrorObserver => "error observer",
            CallableType::PrebuiltType => "prebuilt type",
        };
        write!(f, "{s}")
    }
}
