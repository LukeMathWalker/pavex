//! Specify the routes exposed by your application.
//!
//! # Guide
//!
//! Check out the ["Routing"](https://pavex.dev/docs/guide/routing) section of Pavex's guide
//! for a thorough introduction to routing in Pavex applications.
pub use fallback::RegisteredFallback;
pub use method_guard::{
    ANY, ANY_WITH_EXTENSIONS, CONNECT, DELETE, GET, HEAD, MethodGuard, OPTIONS, PATCH, POST, PUT,
    TRACE,
};
pub use route::RegisteredRoute;
pub use routes::RegisteredRoutes;

mod fallback;
mod method_guard;
mod route;
mod routes;
