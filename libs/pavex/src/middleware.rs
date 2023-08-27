//! Middleware types and utilities.
//!
//! See [`Blueprint::wrap`] and [`Next`] for more information.
//!
//! [`Blueprint::wrap`]: crate::blueprint::Blueprint::wrap
use std::future::IntoFuture;

use crate::response::Response;

/// A handle to trigger the execution of the rest of the request processing pipeline.
///
/// It is used by wrapping middlewares to delegate the processing of the request to the next
/// middleware in the pipeline (or to the request handler).
///
/// Check out [`Blueprint::wrap`] for more information.
///
/// [`Blueprint::wrap`]: crate::blueprint::Blueprint::wrap
pub struct Next<C>
where
    C: IntoFuture<Output = Response>,
{
    request_pipeline: C,
}

impl<C> Next<C>
where
    C: IntoFuture<Output = Response>,
{
    /// Creates a new [`Next`] instance.
    pub fn new(request_pipeline: C) -> Self {
        Self { request_pipeline }
    }
}

impl<C> IntoFuture for Next<C>
where
    C: IntoFuture<Output = Response>,
{
    type Output = Response;
    type IntoFuture = <C as IntoFuture>::IntoFuture;

    fn into_future(self) -> Self::IntoFuture {
        self.request_pipeline.into_future()
    }
}
