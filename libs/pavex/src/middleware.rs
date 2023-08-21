//! Middleware types and utilities.
//!
//! See [`Blueprint::wrap`] and [`Next`] for more information.
//!
//! [`Blueprint::wrap`]: crate::blueprint::Blueprint::wrap
use std::future::IntoFuture;

use crate::response::Response;

/// A [`Future`] that represents the next step in the request processing pipeline.
///
/// It is used by wrapping middlewares to delegate the processing of the request to the next
/// middleware in the pipeline (or to the request handler).
///
/// Check out [`Blueprint::wrap`] for more information.
///
/// [`Blueprint::wrap`]: crate::blueprint::Blueprint::wrap
/// [`Future`]: std::future::Future
pub struct Next<C> {
    request_pipeline: C,
}

impl<C> Next<C> {
    /// Creates a new [`Next`] instance.
    pub fn new(request_pipeline: C) -> Self
    where
        C: IntoFuture<Output = Response>,
    {
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
