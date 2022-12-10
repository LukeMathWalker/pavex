pub use app::App;
pub use diagnostic::CompilerDiagnostic;

mod app;
mod call_graph;
mod codegen;
mod codegen_utils;
mod constructors;
pub(crate) mod dependency_graph;
mod diagnostic;
mod generated_app;
mod resolvers;
mod traits;
