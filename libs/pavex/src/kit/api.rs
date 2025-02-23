use crate::blueprint::Blueprint;
use crate::blueprint::constructor::Constructor;
use crate::blueprint::linter::Lint;
use crate::request::body::{BodySizeLimit, BufferedBody, JsonBody, UrlEncodedBody};
use crate::request::path::PathParams;
use crate::request::query::QueryParams;
use crate::telemetry::ServerRequestId;

#[derive(Clone, Debug)]
#[non_exhaustive]
/// A collection of first-party constructors that are often needed when building APIs.
///
/// # Guide
///
/// Check out the ["Kits"](https://pavex.dev/docs/guide/dependency_injection/kits)
/// section of Pavex's guide for a thorough introduction to kits and how to
/// customize them.
///
/// # Example
///
/// ```rust
/// use pavex::blueprint::Blueprint;
/// use pavex::kit::ApiKit;
///  
/// let mut bp = Blueprint::new();
/// let kit = ApiKit::new().register(&mut bp);
/// ```
pub struct ApiKit {
    /// The [default constructor](PathParams::default_constructor)
    /// for [`PathParams`](struct@PathParams).
    pub path_params: Option<Constructor>,
    /// The [default constructor](QueryParams::default_constructor)
    /// for [`QueryParams`].
    pub query_params: Option<Constructor>,
    /// The [default constructor](JsonBody::default_constructor) for [`JsonBody`].
    pub json_body: Option<Constructor>,
    /// The [default constructor](UrlEncodedBody::default_constructor) for [`UrlEncodedBody`].
    pub url_encoded_body: Option<Constructor>,
    /// The [default constructor](BufferedBody::default_constructor) for [`BufferedBody`].
    pub buffered_body: Option<Constructor>,
    /// The [default constructor](BodySizeLimit::default_constructor) for [`BodySizeLimit`].
    pub body_size_limit: Option<Constructor>,
    /// The [default constructor](ServerRequestId::default_constructor) for [`ServerRequestId`].
    pub server_request_id: Option<Constructor>,
}

impl Default for ApiKit {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiKit {
    /// Create a new [`ApiKit`] with all the bundled constructors.
    pub fn new() -> Self {
        Self {
            path_params: Some(PathParams::default_constructor().ignore(Lint::Unused)),
            query_params: Some(QueryParams::default_constructor().ignore(Lint::Unused)),
            json_body: Some(JsonBody::default_constructor().ignore(Lint::Unused)),
            url_encoded_body: Some(UrlEncodedBody::default_constructor().ignore(Lint::Unused)),
            buffered_body: Some(BufferedBody::default_constructor().ignore(Lint::Unused)),
            body_size_limit: Some(BodySizeLimit::default_constructor().ignore(Lint::Unused)),
            server_request_id: Some(ServerRequestId::default_constructor().ignore(Lint::Unused)),
        }
    }

    /// Register all the bundled constructors with a [`Blueprint`].
    ///
    /// Constructors that are set to `None` will not be registered.
    pub fn register(self, bp: &mut Blueprint) -> RegisteredApiKit {
        if let Some(path_params) = self.path_params {
            path_params.register(bp);
        }
        if let Some(query_params) = self.query_params {
            query_params.register(bp);
        }
        if let Some(json_body) = self.json_body {
            json_body.register(bp);
        }
        if let Some(url_encoded_body) = self.url_encoded_body {
            url_encoded_body.register(bp);
        }
        if let Some(buffered_body) = self.buffered_body {
            buffered_body.register(bp);
        }
        if let Some(body_size_limit) = self.body_size_limit {
            body_size_limit.register(bp);
        }
        if let Some(server_request_id) = self.server_request_id {
            server_request_id.register(bp);
        }
        RegisteredApiKit {}
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
/// The type returned by [`ApiKit::register`].
pub struct RegisteredApiKit {}
