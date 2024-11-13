pub use processed_db::UserComponentDb;
pub use raw_db::{UserComponent, UserComponentId};
pub(crate) use router::{DomainRouter, PathRouter, Router};
pub use scope_graph::{ScopeGraph, ScopeId};

mod processed_db;
mod raw_db;
mod resolved_paths;
mod router;
mod router_key;
mod scope_graph;
