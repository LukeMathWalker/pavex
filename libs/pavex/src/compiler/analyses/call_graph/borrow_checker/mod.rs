//! Ensure that a [`CallGraph`]'s structure allows to generate code that passes the Rust borrow checker.
//!
//! [`OrderedCallGraph`] is the primary entrypoint of this module.
//!
//! [`CallGraph`]: crate::compiler::analyses::call_graph::CallGraph
pub(crate) use ordered_call_graph::OrderedCallGraph;

mod ancestor_consumes_descendant_borrows;
mod assign_order;
mod clone;
mod complex;
mod copy;
mod diagnostic_helpers;
mod multiple_consumers;
mod ordered_call_graph;
mod ownership_relationship;
