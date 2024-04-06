use serde::Deserialize;

use crate::blueprint::constructor::{Constructor, Lifecycle, RegisteredConstructor};
use crate::blueprint::Blueprint;
use crate::f;
use crate::request::path::deserializer::PathDeserializer;
use crate::request::path::errors::{DecodeError, ExtractPathParamsError, InvalidUtf8InPathParam};

use super::RawPathParams;

/// Extract (typed) path parameters from the path of an incoming request.
///
/// # Guide
///
/// Check out [the guide](https://pavex.dev/docs/guide/request_data/path/path_parameters/)
/// for more information on how to use this extractor.
///
/// # Example
///
/// ```rust
/// use pavex::f;
/// use pavex::blueprint::{router::GET, Blueprint, constructor::Lifecycle};
/// use pavex::request::path::PathParams;
///
/// fn blueprint() -> Blueprint {
///     let mut bp = Blueprint::new();
///     // Register the default constructor and error handler for `PathParams`.
///     PathParams::register(&mut bp);
///     // Register a route with a path parameter, `:home_id`.
///     bp.route(GET, "/home/:home_id", f!(crate::get_home));
///     bp
/// }
///
/// // The PathParams attribute macro derives the necessary (de)serialization traits.
/// #[PathParams]
/// pub struct Home {
///     // The name of the field must match the name of the path parameter
///     // used in `bp.route`.
///     home_id: u32
/// }
///
/// // The `PathParams` extractor deserializes the extracted path parameters into
/// // the type you specified—`HomePathParams` in this case.
/// pub fn get_home(params: &PathParams<Home>) -> String {
///    format!("The identifier for this home is: {}", params.0.home_id)
/// }
/// ```
///
/// `home_id` will be set to `1` for an incoming `/home/1` request.  
/// Extraction will fail, instead, if we receive an `/home/abc` request.
#[doc(alias = "Path")]
#[doc(alias = "RouteParams")]
#[doc(alias = "UrlParams")]
pub struct PathParams<T>(
    /// The extracted path parameters, deserialized into `T`, the type you specified.
    pub T,
);

impl<T> PathParams<T> {
    /// The default constructor for [`PathParams`].
    ///
    /// If the extraction fails, an [`ExtractPathParamsError`] is returned.
    pub fn extract<'server, 'request>(
        params: RawPathParams<'server, 'request>,
    ) -> Result<Self, ExtractPathParamsError>
    where
        T: Deserialize<'request>,
        // The parameter ids live as long as the server, while the values are tied to the lifecycle
        // of an incoming request. So it's always true that 'key outlives 'value.
        'server: 'request,
    {
        let mut decoded_params = Vec::with_capacity(params.len());
        for (id, value) in params.iter() {
            let decoded_value = value.decode().map_err(|e| {
                let DecodeError {
                    invalid_raw_segment,
                    source,
                } = e;
                ExtractPathParamsError::InvalidUtf8InPathParameter(InvalidUtf8InPathParam {
                    invalid_key: id.into(),
                    invalid_raw_segment,
                    source,
                })
            })?;
            decoded_params.push((id, decoded_value));
        }
        let deserializer = PathDeserializer::new(&decoded_params);
        T::deserialize(deserializer)
            .map_err(ExtractPathParamsError::PathDeserializationError)
            .map(PathParams)
    }
}

impl PathParams<()> {
    /// Register the [default constructor](Self::default_constructor)
    /// for [`PathParams`] with a [`Blueprint`].
    pub fn register(bp: &mut Blueprint) -> RegisteredConstructor {
        Self::default_constructor().register(bp)
    }

    /// The [default constructor](PathParams::extract)
    /// and [error handler](ExtractPathParamsError::into_response)
    /// for [`PathParams`].
    pub fn default_constructor() -> Constructor {
        Constructor::request_scoped(f!(super::PathParams::extract))
            .error_handler(f!(super::errors::ExtractPathParamsError::into_response))
    }
}
