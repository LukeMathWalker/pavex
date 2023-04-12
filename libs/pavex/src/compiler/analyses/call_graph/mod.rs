pub(crate) use application_state::{application_state_call_graph, ApplicationStateCallGraph};
pub(crate) use core_graph::{
    CallGraph, CallGraphEdgeMetadata, CallGraphNode, NumberOfAllowedInvocations,
};
pub(crate) use request_handler::handler_call_graph;

mod application_state;
mod borrow_checker;
mod codegen;
mod core_graph;
mod request_handler;
