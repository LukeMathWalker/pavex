//! Execute common logic across multiple routes.  
//!
//! # Guide
//!
//! Check out the ["Middleware"](https://pavex.dev/docs/guide/middleware) section of Pavex's guide
//! for a thorough introduction to middlewares in Pavex applications.
mod post;
mod pre;
mod wrapping;

pub use post::{PostProcessingMiddleware, RegisteredPostProcessingMiddleware};
pub use wrapping::{RegisteredWrappingMiddleware, WrappingMiddleware};
