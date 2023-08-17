//! Low-level tools for building and manipulating [`Response`](crate::response::Response) bodies.
//!
//! Primarily useful if you are working with
//! [`Response::set_raw_body`](crate::response::Response::set_raw_body).
pub use bytes::{Bytes, BytesMut};
/// Trait representing a streaming [`Response`](crate::response::Response) body.  
///
pub use http_body::Body as RawBody;
pub use http_body::{Empty, Full};

/// A type-erased streaming [`Response`](crate::response::Response) body. The
/// most common body type in Pavex.
///
pub type BoxBody = http_body::combinators::BoxBody<Bytes, crate::Error>;
pub use boxed::boxed;

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
    use super::{BoxBody, Bytes, RawBody};
    use crate::Error;

    /// Convert a [`RawBody`] into a [`BoxBody`].
    pub fn boxed<B>(body: B) -> BoxBody
    where
        B: RawBody<Data = Bytes> + Send + Sync + 'static,
        B::Error: Into<Box<dyn std::error::Error + Sync + Send>>,
    {
        try_downcast(body).unwrap_or_else(|body| body.map_err(Error::new).boxed())
    }

    fn try_downcast<T, K>(k: K) -> Result<T, K>
    where
        T: 'static,
        K: Send + 'static,
    {
        let mut k = Some(k);
        if let Some(k) = <dyn std::any::Any>::downcast_mut::<Option<T>>(&mut k) {
            Ok(k.take().unwrap())
        } else {
            Err(k.unwrap())
        }
    }

    #[test]
    fn test_try_downcast() {
        assert_eq!(try_downcast::<i32, _>(5_u32), Err(5_u32));
        assert_eq!(try_downcast::<i32, _>(5_i32), Ok(5_i32));
    }
}
