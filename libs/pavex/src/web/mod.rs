pub use app::App;
pub use diagnostic::CompilerDiagnostic;

mod app;
mod call_graph;
mod codegen;
mod codegen_utils;
mod constructors;
mod diagnostic;
mod error_handlers;
mod generated_app;
mod resolvers;
mod traits;
mod utils;
