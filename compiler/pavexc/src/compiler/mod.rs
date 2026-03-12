#![allow(clippy::too_many_arguments)]

pub use app::App;

mod analyses;
mod app;
mod codegen;
mod codegen_utils;
mod component;
mod computation;
mod framework_rustdoc;
mod generated_app;
mod interner;
mod path_parameters;
mod traits;
