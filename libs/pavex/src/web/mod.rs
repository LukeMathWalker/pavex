pub use app::App;
pub use diagnostic::CompilerDiagnostic;

mod app;
mod application_state_call_graph;
mod codegen;
mod codegen_utils;
pub(crate) mod dependency_graph;
mod diagnostic;
mod generated_app;
mod handler_call_graph;
