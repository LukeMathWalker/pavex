use crate::blueprint::constructor::Constructor;
use crate::blueprint::Blueprint;
use crate::request::body::{BodySizeLimit, BufferedBody, JsonBody};
use crate::request::path::PathParams;
use crate::request::query::QueryParams;

#[derive(Clone, Debug)]
#[non_exhaustive]
/// A collection of first-party constructors that are often needed when building APIs.
///
/// # Default
///
/// ```rust
/// use pavex::blueprint::Blueprint;
/// use pavex::kit::ApiKit;
///  
/// let mut bp = Blueprint::new();
/// let kit = ApiKit::new().register(&mut bp);
/// ```
///
/// # Skip constructors
///
/// You can skip registering some of the bundled constructors by setting their
/// corresponding fields to `None`:
///
/// ```rust
/// use pavex::blueprint::Blueprint;
/// use pavex::kit::ApiKit;
///
/// let mut bp = Blueprint::new();
/// let kit = ApiKit {
///     // The constructor for `PathParams` will not be registered.
///     path_params: None,
///     ..ApiKit::new()
/// };
/// kit.register(&mut bp);
/// ```
///
/// # Customize constructors
///
/// You can also customize the bundled constructors before registering them with
/// a [`Blueprint`]:
///
/// ```rust
/// use pavex::blueprint::Blueprint;
/// use pavex::blueprint::constructor::CloningStrategy;
/// use pavex::kit::ApiKit;
///
/// let mut bp = Blueprint::new();
/// let mut kit = ApiKit::new();
/// if let Some(mut buffered_body) = &mut kit.buffered_body {
///     buffered_body = buffered_body.cloning(CloningStrategy::CloneIfNecessary);
/// }
/// kit.register(&mut bp);
/// ```
pub struct ApiKit {
    /// The default constructor for [`PathParams`](PathParams).
    pub path_params: Option<Constructor>,
    /// The default constructor for [`QueryParams`](QueryParams).
    pub query_params: Option<Constructor>,
    /// The default constructor for [`JsonBody`](JsonBody).
    pub json_body: Option<Constructor>,
    /// The default constructor for [`BufferedBody`](BufferedBody).
    pub buffered_body: Option<Constructor>,
    /// The default constructor for [`BodySizeLimit`](BodySizeLimit).
    pub body_size_limit: Option<Constructor>,
}

impl ApiKit {
    /// Create a new [`ApiKit`] with all the bundled constructors.
    pub fn new() -> Self {
        Self {
            path_params: Some(PathParams::default_constructor()),
            query_params: Some(QueryParams::default_constructor()),
            json_body: Some(JsonBody::default_constructor()),
            buffered_body: Some(BufferedBody::default_constructor()),
            body_size_limit: Some(BodySizeLimit::default_constructor()),
        }
    }

    /// Register all the bundled constructors with a [`Blueprint`].
    ///
    /// Constructors that are set to `None` will not be registered—see
    /// [`ApiKit`]'s documentation for more information.
    pub fn register(self, bp: &mut Blueprint) {
        if let Some(path_params) = self.path_params {
            path_params.register(bp);
        }
        if let Some(query_params) = self.query_params {
            query_params.register(bp);
        }
        if let Some(json_body) = self.json_body {
            json_body.register(bp);
        }
        if let Some(buffered_body) = self.buffered_body {
            buffered_body.register(bp);
        }
        if let Some(body_size_limit) = self.body_size_limit {
            body_size_limit.register(bp);
        }
    }
}
