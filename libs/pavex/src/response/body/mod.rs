//! Tools for building and manipulating [`Response`](crate::response::Response) bodies.
//!
//! Check out [`Response::set_typed_body`] for more details.
//!
//! [`Response::set_typed_body`]: crate::response::Response::set_typed_body
mod bytes;
mod html;
mod json;
mod plain_text;
pub mod raw;

pub mod errors;

pub use html::Html;
pub use json::Json;
pub use typed_body::TypedBody;

mod typed_body {
    use super::raw::Bytes;
    use super::raw::RawBody;
    use crate::http::HeaderValue;

    /// A trait that ties together a [`Response`] body with
    /// its expected `Content-Type` header.
    ///
    /// Check out [`Response::set_typed_body`](crate::response::Response) for more details
    /// on `TypedBody` is leveraged when building a [`Response`].
    ///
    /// # Implementing `TypedBody`
    ///
    /// You might find yourself implementing `TypedBody` if none of the implementations
    /// provided out-of-the-box by Pavex in the [`body`](super) module satisfies your needs.
    ///
    /// You need to specify two things:
    ///
    /// 1. The value of the `Content-Type` header
    /// 2. The low-level representation of your body type
    ///
    /// Let's focus on 2., the trickier bit. You'll be working with the types in
    /// the [`body::raw`](super::raw) module.  
    ///
    /// ## Buffered body
    ///
    /// [`Full<Bytes>`](super::raw::Full) is the "canonical" choice if your body is fully
    /// buffered in memory before being transmitted over the network.  
    /// You need to convert your body type into a buffer ([`Bytes`](super::raw::Bytes))
    /// which is then wrapped in [`Full`](super::raw::Full) to signal that the entire
    /// body is a single "chunk".  
    ///
    /// Let's see how you could implement `TypedBody` for a `String` wrapper
    /// as a reference example:
    ///
    /// ```rust,
    /// use pavex::http::HeaderValue;
    /// use pavex::response::body::{
    ///     TypedBody,
    ///     raw::{Full, Bytes}
    /// };
    ///
    /// struct MyString(String);
    ///
    /// impl TypedBody for MyString {
    ///     type Body = Full<Bytes>;
    ///
    ///     fn content_type(&self) -> HeaderValue {
    ///         HeaderValue::from_static("text/plain; charset=utf-8")
    ///     }
    ///
    ///     fn body(self) -> Self::Body {
    ///         Full::new(self.0.into())
    ///     }
    /// }
    /// ```
    ///
    /// ## Streaming body
    ///
    /// Streaming bodies are trickier.  
    /// You might need to implement [`RawBody`] directly for your body type.  
    ///
    /// [`Response`]: crate::response::Response
    // TODO: expand guide for streaming bodies.
    pub trait TypedBody {
        type Body: RawBody<Data = Bytes> + Send + Sync + 'static;

        /// The header value that should be used as `Content-Type` when
        /// returning this [`Response`](crate::response::Response).
        fn content_type(&self) -> HeaderValue;

        /// The actual body type.
        ///
        /// It must implement the [`RawBody`] trait.
        fn body(self) -> Self::Body;
    }
}
