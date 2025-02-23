use std::pin::{Pin, pin};
use std::task::{Context, Poll};

use http_body::{Frame, SizeHint};
use http_body_util::combinators::UnsyncBoxBody;

use crate::response::body::body_::boxed::boxed;
use crate::response::body::raw::RawBody;

use super::raw::Bytes;

/// The body type used in Pavex's [`Response`](crate::response::Response)s.
///
/// # Low-level
///
/// You'll rarely have to work with `ResponseBody` directly.  
/// Rely on [`Response::set_typed_body`] and [`Response::set_raw_body`] to
/// build the body of your responses.  
/// `ResponseBody` is part of the public API to give a name to the type returned by
/// [`Response::body`] and [`Response::body_mut`].
///
/// [`Response::set_typed_body`]: crate::response::Response::set_typed_body
/// [`Response::set_raw_body`]: crate::response::Response::set_raw_body
/// [`Response::body`]: crate::response::Response::body
/// [`Response::body_mut`]: crate::response::Response::body_mut
#[derive(Debug)]
pub struct ResponseBody(UnsyncBoxBody<Bytes, crate::Error>);

impl ResponseBody {
    /// Create a new [`ResponseBody`] from a raw body type.
    pub fn new<B>(body: B) -> Self
    where
        B: RawBody<Data = Bytes> + Send + 'static,
        <B as RawBody>::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        boxed(body)
    }
}

impl RawBody for ResponseBody {
    type Data = Bytes;
    type Error = crate::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        pin!(&mut self.0).as_mut().poll_frame(cx)
    }

    fn is_end_stream(&self) -> bool {
        self.0.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.0.size_hint()
    }
}

impl Default for ResponseBody {
    fn default() -> Self {
        ResponseBody::new(super::raw::Empty::new())
    }
}

// Most of this module is a direct copy (with, from time to time,
// minor modifications) of the corresponding `body` module in
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
mod boxed {
    //! Body types and utilities used by Pavex.
    use http_body_util::BodyExt;

    use crate::Error;
    use crate::response::body::raw::RawBody;

    use super::{Bytes, ResponseBody};

    /// Convert a [`RawBody`] into a [`ResponseBody`].
    pub(super) fn boxed<B>(body: B) -> ResponseBody
    where
        B: RawBody<Data = Bytes> + Send + 'static,
        B::Error: Into<Box<dyn std::error::Error + Sync + Send>>,
    {
        ResponseBody(
            try_downcast(body).unwrap_or_else(|body| body.map_err(Error::new).boxed_unsync()),
        )
    }

    fn try_downcast<T, K>(k: K) -> Result<T, K>
    where
        T: 'static,
        K: Send + 'static,
    {
        let mut k = Some(k);
        match <dyn std::any::Any>::downcast_mut::<Option<T>>(&mut k) {
            Some(k) => Ok(k.take().unwrap()),
            _ => Err(k.unwrap()),
        }
    }

    #[test]
    fn test_try_downcast() {
        assert_eq!(try_downcast::<i32, _>(5_u32), Err(5_u32));
        assert_eq!(try_downcast::<i32, _>(5_i32), Ok(5_i32));
    }
}
