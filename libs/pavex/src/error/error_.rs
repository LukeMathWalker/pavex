// Most of this module is a direct copy (with, from time to time,
// minor modifications) of the corresponding `error` module in
// `axum-core`.
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
use crate::{methods, response::Response};
use std::fmt;

/// Pavex's error type: an opaque wrapper around the concrete error type
/// return by your components (e.g. request handlers, constructors, etc.).
/// It is used as an input parameter by
/// [error observers](https://pavex.dev/docs/guide/errors/error_observers/) and
/// [universal error handlers](https://pavex.dev/docs/guide/errors/error_handlers/#universal).
///
/// # Guide
///
/// Check out [the guide](https://pavex.dev/docs/guide/errors/)
/// for a thorough introduction to error handling and reporting in Pavex.
///
/// # Implementation details
///
/// It's a thin shim over `Box<dyn std::error::Error + Send + Sync>`.
#[derive(Debug)]
pub struct Error {
    inner: Box<dyn std::error::Error + Send + Sync>,
}

#[methods]
impl Error {
    /// Create a new [`Error`] from a boxable error.
    pub fn new<E>(error: E) -> Self
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Self {
            inner: error.into(),
        }
    }

    /// Convert [`Error`] back into the underlying boxed error.
    pub fn into_inner(self) -> Box<dyn std::error::Error + Send + Sync> {
        self.inner
    }

    /// Return a reference to the underlying boxed error.
    pub fn inner_ref(&self) -> &(dyn std::error::Error + Send + Sync) {
        &*self.inner
    }

    /// Return an opaque `500 Internal Server Error` to the caller.
    ///
    /// It is used as the default error handler for [`pavex::Error`][`Error`].
    ///
    /// # Guide
    ///
    /// Check out [the "Fallback error handler" section of the guide](https://pavex.dev/docs/guide/errors/error_handlers/#fallback-error-handler)
    /// for more details on the special role played by the error handler for [`pavex::Error`][`Error`] in Pavex.
    #[error_handler(pavex = crate)]
    pub fn to_response(&self) -> Response {
        Response::internal_server_error()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.inner)
    }
}
