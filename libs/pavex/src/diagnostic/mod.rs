//! A toolkit to assemble and report errors and warnings to the user.
use std::fmt::{Display, Formatter};

pub(crate) use compiler_diagnostic::CompilerDiagnostic;
pub(crate) use miette_utils::{
    convert_proc_macro_span, convert_rustdoc_span, OptionalSourceSpanExt, SourceSpanExt,
};
pub(crate) use proc_macro_utils::ProcMacroSpanExt;
pub(crate) use registration_locations::get_f_macro_invocation_span;
pub(crate) use source_file::{read_source_file, LocationExt, ParsedSourceFile};

mod compiler_diagnostic;
mod miette_utils;
mod proc_macro_utils;
mod registration_locations;
mod source_file;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub(crate) enum CallableType {
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
