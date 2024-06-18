#![allow(clippy::too_many_arguments)]

pub use app::App;

mod analyses;
mod app;
mod codegen;
mod codegen_utils;
mod component;
mod computation;
mod generated_app;
mod interner;
mod path_parameter_validation;
// HACK: breaking encapsulation because resolver logic is split across this module
// and `resolved_path` in `language`.
pub mod resolvers;
mod traits;
mod utils;
