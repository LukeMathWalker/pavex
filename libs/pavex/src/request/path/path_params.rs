use pavex_macros::methods;
use serde::Deserialize;

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
/// use pavex::{get, request::path::PathParams};
///
/// // Define a route with a path parameter, `{home_id}`.
/// // The `PathParams` extractor deserializes the extracted path parameters into
/// // the type you specified—`Home` in this case.
/// #[get(path = "/home/{home_id}")]
/// pub fn get_home(params: &PathParams<Home>) -> String {
///    format!("The identifier for this home is: {}", params.0.home_id)
/// }
///
/// // The PathParams attribute macro derives the necessary (de)serialization traits.
/// #[PathParams]
/// pub struct Home {
///     // The name of the field must match the name of the path parameter
///     // used in the route definition.
///     home_id: u32
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

#[methods]
impl<T> PathParams<T> {
    /// The default constructor for [`PathParams`].
    ///
    /// If the extraction fails, an [`ExtractPathParamsError`] is returned.
    #[request_scoped]
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
