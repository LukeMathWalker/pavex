//! A toolkit to assemble and report errors and warnings to the user.
pub use compiler_diagnostic::{CompilerDiagnostic, CompilerDiagnosticBuilder};
pub use miette_utils::{
    convert_proc_macro_span, convert_rustdoc_span, OptionalSourceSpanExt, SourceSpanExt,
};
pub use proc_macro_utils::ProcMacroSpanExt;
pub use registration_locations::{
    get_f_macro_invocation_span, get_registration_location,
    get_registration_location_for_a_request_handler,
};
pub use source_file::{read_source_file, LocationExt, ParsedSourceFile};

mod compiler_diagnostic;
mod miette_utils;
mod proc_macro_utils;
mod registration_locations;
mod source_file;
