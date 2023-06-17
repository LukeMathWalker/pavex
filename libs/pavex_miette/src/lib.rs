//! This crate provides a custom graphical Miette handler for the Pavex project.
//!
//! The handler is largely based on the `miette::handlers::GraphicalHandler`, with
//! one key difference: we only report the code snippets from the related errors associated
//! with a report.
//! We have also done other minor tweaks to the graphical layout to better suit our needs.
//!
//! This allows us to display snippets that come from different source files, a feature
//! that doesn't have first-class support in `miette`.  
//! In other words, you can see this custom handler as a "hack" to avoid having to
//! maintain a full fork of `miette`.
pub use graphical_report_handler::GraphicalReportHandler;
pub use opts::{PavexMietteHandler, PavexMietteHandlerOpts};

mod diagnostic_chain;
mod graphical_report_handler;
mod opts;
