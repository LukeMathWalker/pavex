//! Execute common logic across multiple routes.  
//!
//! Check out [`Blueprint::wrap`] for a brief introduction to wrapping middlewares in Pavex.
//!
//! [`Blueprint::wrap`]: crate::blueprint::Blueprint::wrap
mod registered;
mod unregistered;

pub use registered::RegisteredWrappingMiddleware;
pub use unregistered::WrappingMiddleware;
