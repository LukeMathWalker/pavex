//! Middleware types and utilities.
//!
//! # Guide
//!
//! Check out the ["Middleware"](https://pavex.dev/docs/guide/middleware)
//! section of Pavex's guide for a thorough introduction to middlewares
//! in Pavex applications.
use std::future::IntoFuture;

use crate::response::{IntoResponse, Response};

/// A handle to trigger the execution of the rest of the request processing pipeline.
///
/// It is used by wrapping middlewares to delegate the processing of the request to the next
/// middleware in the pipeline (or to the request handler).
///
/// Check out [`Blueprint::wrap`] for more information.
///
/// [`Blueprint::wrap`]: crate::Blueprint::wrap
pub struct Next<C>
where
    C: IntoFuture<Output = Response>,
{
    request_pipeline: C,
}

/// The return type of a pre-processing middleware.
///
/// It signals to Pavex whether the request processing should continue or be aborted,
/// and if so, with what response.
///
/// Check out [`Blueprint::pre_process`] for more information.
///
/// [`Blueprint::pre_process`]: crate::Blueprint::pre_process
pub enum Processing<T = Response>
where
    T: IntoResponse,
{
    Continue,
    EarlyReturn(T),
}

impl<T: IntoResponse> Processing<T> {
    /// Converts the [`Processing`] instance into a response, if the intention is to abort.
    /// It returns `None` if the intention is to continue the request processing.
    pub fn into_response(self) -> Option<T> {
        match self {
            Processing::Continue => None,
            Processing::EarlyReturn(response) => Some(response),
        }
    }
}

/// A wrapping middleware that...does nothing.
///
/// It just invokes the next stage in the request processing pipeline and returns its result.
#[doc(hidden)]
pub async fn wrap_noop<C: IntoFuture<Output = Response>>(next: Next<C>) -> Response {
    next.await
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
