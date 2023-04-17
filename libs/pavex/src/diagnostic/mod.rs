//! A toolkit to assemble and report errors and warnings to the user.
use std::fmt::{Display, Formatter};

pub(crate) use compiler_diagnostic::{
    AnnotatedSnippet, CompilerDiagnostic, CompilerDiagnosticBuilder, HelpWithSnippet,
};
pub(crate) use ordinals::ZeroBasedOrdinal;
pub(crate) use proc_macro_utils::ProcMacroSpanExt;
pub(crate) use registration_locations::{
    get_bp_new_span, get_f_macro_invocation_span, get_nest_at_prefix_span, get_route_path_span,
};
pub(crate) use source_file::{read_source_file, LocationExt, ParsedSourceFile};

pub(crate) use self::miette::{
    convert_proc_macro_span, convert_rustdoc_span, OptionalSourceSpanExt, SourceSpanExt,
};

mod compiler_diagnostic;
mod miette;
mod ordinals;
mod proc_macro_utils;
mod registration_locations;
mod source_file;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum CallableType {
    RequestHandler,
    Constructor,
    ErrorHandler,
}

impl Display for CallableType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CallableType::RequestHandler => "request handler",
            CallableType::Constructor => "constructor",
            CallableType::ErrorHandler => "error handler",
        };
        write!(f, "{s}")
    }
}
