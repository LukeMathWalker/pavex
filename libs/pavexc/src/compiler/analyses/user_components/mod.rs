pub use annotations::AnnotatedItemId;
pub use component::{ErrorHandlerTarget, UserComponent, UserComponentId};
pub use db::UserComponentDb;
pub(crate) use router::{DomainRouter, PathRouter, Router};
pub use scope_graph::{ScopeGraph, ScopeId};
pub use source::UserComponentSource;

mod annotations;
mod auxiliary;
mod blueprint;
mod component;
mod db;
mod imports;
mod paths;
mod router;
mod router_key;
mod scope_graph;
mod source;
