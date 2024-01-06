use std::pin::Pin;
use std::task::{Context, Poll};

use http_body::{Body, Frame};
use hyper::body::Incoming;
use pin_project_lite::pin_project;

pin_project! {
    #[derive(Debug)]

    /// The raw body of the incoming HTTP request.
    ///
    /// # Guide
    ///
    /// You're looking at the stream of bytes coming from the network.
    /// There are **no safeguards nor conveniences**.
    ///
    /// Check out [the guide](https://pavex.dev/docs/guide/request_data/wire_data/)
    /// for a thorough introduction to `RawIncomingBody` and guidance on when to use it.
    pub struct RawIncomingBody {
        #[pin] inner: Incoming,
    }
}

// We just delegate to the underlying `Incoming` `Body` implementation.
impl Body for RawIncomingBody {
    type Data = <Incoming as Body>::Data;
    type Error = <Incoming as Body>::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        self.project().inner.poll_frame(cx)
    }
}

impl From<Incoming> for RawIncomingBody {
    fn from(inner: Incoming) -> Self {
        Self { inner }
    }
}
