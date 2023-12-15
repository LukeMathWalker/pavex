//! Specify the routes exposed by your application.  
//!
//! Check out [`Blueprint::route`] for a brief introduction to request routing in Pavex.
//!
//! [`Blueprint::route`]: crate::blueprint::Blueprint::route
pub use http::Method;

pub use fallback::Fallback;
pub use method_guard::{
    MethodGuard, ANY, ANY_WITH_EXTENSIONS, CONNECT, DELETE, GET, HEAD, OPTIONS, PATCH, POST, PUT,
    TRACE,
};
pub use route::Route;

mod fallback;
mod method_guard;
mod route;
