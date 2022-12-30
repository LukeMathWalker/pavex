// Most of this module is an adaptation of the corresponding
// module in `axum-core`
//
// Copyright (c) 2019 Axum Contributors
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
use std::borrow::Cow;

use bytes::{Bytes, BytesMut};
use http::{header, HeaderValue, StatusCode};
use http_body::combinators::{MapData, MapErr};
use http_body::{Empty, Full};

use crate::body::BoxBody;

/// Type alias for `http::Response`.
/// The generic parameter for the body type defaults to `BoxBody`, the most common body
/// type used in `pavex`.
pub type Response<T = BoxBody> = http::Response<T>;

/// Convert a type into an HTTP response.
///
/// Types that implement `IntoResponse` can be returned:
///
/// - as the output type of an infallible route handler,
///   e.g. `fn handler() -> T` where `T: IntoResponse`.
/// - as the `Ok` variant of the `Result` returned by a fallible route handler,
///   e.g. `fn handler() -> Result<T, E>` where `T: IntoResponse`.
//
// # Implementation notes
//
// This is our primary divergence from `axum-core`'s API: we do NOT implement
// `IntoResponse` for `Result<T, E>` if `T: IntoResponse` and `E: IntoResponse`.
// It would create ambiguity: how should I handle errors? Do I need to implement
// `IntoResponse` for `E`? Do I need to specify an error handler via `pavex_builder`?
// What if I do both, what gets invoked?
//
// ## Other minor divergences
//
// We are more conservative in the range of types that we implement `IntoResponse` for.
// In particular, no tuples and no `()`.
pub trait IntoResponse {
    /// Convert `self` into an HTTP response.
    fn into_response(self) -> Response;
}

impl IntoResponse for Empty<Bytes> {
    fn into_response(self) -> Response {
        Response::new(crate::body::boxed(self))
    }
}

impl IntoResponse for StatusCode {
    fn into_response(self) -> Response {
        let mut res = Empty::new().into_response();
        *res.status_mut() = self;
        res
    }
}

impl<B> IntoResponse for Response<B>
where
    B: http_body::Body<Data = Bytes> + Send + Sync + 'static,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
{
    fn into_response(self) -> Response {
        self.map(crate::body::boxed)
    }
}

impl IntoResponse for http::response::Parts {
    fn into_response(self) -> Response {
        Response::from_parts(self, crate::body::boxed(Empty::new()))
    }
}

impl IntoResponse for Full<Bytes> {
    fn into_response(self) -> Response {
        Response::new(crate::body::boxed(self))
    }
}

impl<E> IntoResponse for http_body::combinators::BoxBody<Bytes, E>
where
    E: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
{
    fn into_response(self) -> Response {
        Response::new(crate::body::boxed(self))
    }
}

impl<B, F> IntoResponse for MapData<B, F>
where
    B: http_body::Body + Send + Sync + 'static,
    F: FnMut(B::Data) -> Bytes + Send + Sync + 'static,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    fn into_response(self) -> Response {
        Response::new(crate::body::boxed(self))
    }
}

impl<B, F, E> IntoResponse for MapErr<B, F>
where
    B: http_body::Body<Data = Bytes> + Send + Sync + 'static,
    F: FnMut(B::Error) -> E + Send + Sync + 'static,
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    fn into_response(self) -> Response {
        Response::new(crate::body::boxed(self))
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> Response {
        Cow::Borrowed(self).into_response()
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Response {
        Cow::<'static, str>::Owned(self).into_response()
    }
}

impl IntoResponse for Cow<'static, str> {
    fn into_response(self) -> Response {
        let mut res = Full::from(self).into_response();
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
        );
        res
    }
}

impl IntoResponse for Bytes {
    fn into_response(self) -> Response {
        let mut res = Full::from(self).into_response();
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static(mime::APPLICATION_OCTET_STREAM.as_ref()),
        );
        res
    }
}

impl IntoResponse for BytesMut {
    fn into_response(self) -> Response {
        self.freeze().into_response()
    }
}

impl IntoResponse for &'static [u8] {
    fn into_response(self) -> Response {
        Cow::Borrowed(self).into_response()
    }
}

impl IntoResponse for Vec<u8> {
    fn into_response(self) -> Response {
        Cow::<'static, [u8]>::Owned(self).into_response()
    }
}

impl IntoResponse for Cow<'static, [u8]> {
    fn into_response(self) -> Response {
        let mut res = Full::from(self).into_response();
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static(mime::APPLICATION_OCTET_STREAM.as_ref()),
        );
        res
    }
}
