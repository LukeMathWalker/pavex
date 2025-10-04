use bytes::Bytes;
use http::StatusCode;
use http_body_util::Empty;

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
use crate::{
    Response,
    response::{ResponseBody, ResponseHead},
};

/// Convert a type into a [`Response`].
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
// `IntoResponse` for `E`? Do I need to specify an error handler in the blueprint?
// What if I do both, what gets invoked?
//
// ## Other divergences
//
// We are more conservative in the range of types that we implement `IntoResponse` for.
// In particular, no tuples, no `()`, no types for which we'd have to infer a status code.
pub trait IntoResponse {
    /// Convert `self` into an HTTP response.
    fn into_response(self) -> Response;
}

impl<B> IntoResponse for http::Response<B>
where
    B: http_body::Body<Data = Bytes> + Send + Sync + 'static,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
{
    fn into_response(self) -> Response {
        let r: Response = self.into();
        r.into_response()
    }
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

impl IntoResponse for StatusCode {
    fn into_response(self) -> Response {
        Response::new(self)
    }
}

impl IntoResponse for http::response::Parts {
    fn into_response(self) -> Response {
        http::Response::from_parts(self, ResponseBody::new(Empty::new())).into()
    }
}

impl IntoResponse for ResponseHead {
    fn into_response(self) -> Response {
        Response::from_parts(self, ResponseBody::default())
    }
}
