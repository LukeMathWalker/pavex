// Re-export the dependencies that we use in the generated application code.
pub use anyhow::Error;
pub use http;
pub use hyper;
use hyper::body::Bytes;
use hyper::body::HttpBody;
pub use matchit as routing;

use http_body::combinators::BoxBody;

// A dirty hack to make sure that `pavex_runtime` ends up in the generated
// Cargo.toml
pub struct Placeholder;

pub fn box_response_body<B, E>(
    response: http::Response<B>,
) -> http::Response<BoxBody<Bytes, Box<dyn std::error::Error + Send + Sync>>>
where
    B: HttpBody<Data = Bytes, Error = E> + Send + Sync + 'static,
    E: Into<Box<dyn std::error::Error + Send + Sync>> + Send + Sync + 'static,
{
    let (head, body) = response.into_parts();
    let body = BoxBody::new(body.map_err(|e| e.into()));
    http::Response::from_parts(head, body)
}
