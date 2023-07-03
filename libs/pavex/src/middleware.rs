//! Middleware types and utilities.
//! 
//! See [`Blueprint::wrap`] and [`Next`] for more information.
//! 
//! [`Blueprint::wrap`]: crate::blueprint::Blueprint::wrap
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;

use crate::response::Response;

pin_project! {
    /// A [`Future`] that represents the next step in the request processing pipeline.
    /// 
    /// It is used by wrapping middlewares to delegate the processing of the request to the next
    /// middleware in the pipeline (or to the request handler).
    /// 
    /// Check out [`Blueprint::wrap`] for more information.
    /// 
    /// [`Blueprint`]: crate::blueprint::Blueprint
    pub struct Next<C> {
        #[pin]
        request_pipeline: C,
    }
}

impl<C> Next<C>
where
    C: Future<Output = Response>,
{
    /// Creates a new [`Next`] instance.
    pub fn new(request_pipeline: C) -> Self {
        Self { request_pipeline }
    }
}

impl<C> Future for Next<C>
where
    C: Future<Output = Response>,
{
    type Output = Response;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.request_pipeline.poll(cx)
    }
}
