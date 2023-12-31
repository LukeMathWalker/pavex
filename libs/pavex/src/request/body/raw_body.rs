use std::pin::Pin;
use std::task::{Context, Poll};

use http_body::{Body, Frame};
use hyper::body::Incoming;
use pin_project_lite::pin_project;

pin_project! {
    /// The raw body of an incoming HTTP request.
    ///
    /// It represents the stream of [`Bytes`](bytes::Bytes) received from the network.
    ///
    /// # Framework primitive
    ///
    /// `RawIncomingBody` is a framework primitive—you don't need to register any constructor
    /// with `Blueprint` to use it in your application.
    ///
    /// # ⚠️ Warning
    ///
    /// You need to careful when working with `RawIncomingBody`: it doesn't provide any safeguards
    /// against malicious clients.
    /// You should generally prefer [`BufferedBody`] instead, which enforces a size limit on the
    /// incoming body.
    ///
    /// [`BufferedBody`]: crate::request::body::BufferedBody
    #[derive(Debug)]
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
