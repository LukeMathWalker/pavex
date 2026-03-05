//! Orchestrate `cargo rustdoc` invocations to generate JSON documentation.

mod format;
mod orchestration;
mod package_id_spec;
mod progress;
mod toolchain;

pub use orchestration::{CannotGetCrateData, compute_crate_docs};
pub use progress::{ComputeProgress, NoProgress};
