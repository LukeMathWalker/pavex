/// Extract (raw) route parameters from the URL of an incoming request.
///
/// # Example
///
/// ```rust
/// use pavex_builder::{f, router::GET, Blueprint};
/// use pavex_runtime::extract::route::RawRouteParams;
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // [...]
/// // Register a route with a few route parameters.
/// bp.route(GET, "/address/:address_id/home/:home_id", f!(crate::get_home));
/// # }
///
/// fn get_home(params: &RawRouteParams) -> String {
///     let home_id = &params.get("home_id").unwrap();
///     let street_id = &params.get("street_id").unwrap();
///     format!("The home with id {} is in street {}", home_id, street_id)
/// }
/// ```
///
/// `RawRouteParams` is a framework primitive—you don't need to register any constructor
/// with `Blueprint` to use it in your application.
///
/// # What does "raw" mean?
///
/// Route parameters are URL segments, therefore they must comply with the restrictions that apply
/// to the URL itself. In particular, they can only use ASCII characters.  
/// In order to support non-ASCII characters, route parameters are
/// [percent-encoded](https://www.w3schools.com/tags/ref_urlencode.ASP).  
/// If you want to send "123 456" as a route parameter, you have to percent-encode it: it becomes
/// "123%20456" since "%20" is the percent-encoding for a space character.
///
/// `RawRouteParams` gives you access to the **raw** route parameters, i.e. the route parameters
/// as they are extracted from the URL, before any kind of processing has taken
/// place.
///
/// In particular, `RawRouteParams` does **not** perform any percent-decoding.  
/// If you send a request to `/address/123%20456/home/789`, the `RawRouteParams` for
/// `/address/:address_id/home/:home_id` will contain the following key-value pairs:
///
/// - `address_id`: `123%20456`
/// - `home_id`: `789`
///
/// `address_id` is not `123 456` because `RawRouteParams` does not perform percent-decoding!
/// Therefore `%20` is not interpreted as a space character.
///
/// There are situations where you might want to work with the raw route parameters, but
/// most of the time you'll want to use [`RouteParams`] instead—it performs percent-decoding
/// and deserialization for you.
#[doc(inline)]
pub use matchit::Params as RawRouteParams;
use percent_encoding::percent_decode_str;
use serde::Deserialize;

use crate::extract::route::deserializer::PathDeserializer;
use crate::extract::route::errors::{ExtractRouteParamsError, InvalidUtf8InPathParam};

/// Extract (typed) route parameters from the URL of an incoming request.
///
/// # Sections
///
/// - [Example](#example)
/// - [Supported types](#supported-types)
/// - [Unsupported types](#unsupported-types)
/// - [Working with raw route parameters](#working-with-raw-route-parameters)
///
/// # Example
///
/// ```rust
/// use pavex_builder::{f, router::GET, Blueprint, Lifecycle};
/// use pavex_runtime::extract::route::RouteParams;
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // Register the default constructor and error handler for `RouteParams`.
/// bp.constructor(
///     f!(pavex_runtime::extract::path::RouteParams::extract),
///     Lifecycle::RequestScoped,
/// ).error_handler(
///     f!(pavex_runtime::extract::path::ExtractRouteParamsError::into_response)
/// );
/// // Register a route with a route parameter, `:home_id`.
/// bp.route(GET, "/home/:home_id", f!(crate::get_home));
/// # }
///
/// #[derive(serde::Deserialize)]
/// struct HomeRouteParams {
///     // The name of the field must match the name of the route parameter
///     // used in `bp.route`.
///     home_id: u32
/// }
///
/// // The `RouteParams` extractor will deserialize the route parameters into
/// // the type you specified—`HomeRouteParams` in this case.
/// fn get_home(params: &RouteParams<HomeRouteParams>) -> String {
///    format!("The identifier for this home is: {}", params.0.home_id)
/// }
/// ```
///
/// `home_id` will be set to `1` for an incoming `/home/1` request.  
/// Extraction will fail, instead, if we receive an `/home/abc` request.
///
/// # Supported types
///
/// `T` in `RouteParams<T>` must implement [`serde::Deserialize`].  
/// `T` must be struct with named fields, where each field name matches one of the route parameter
/// names used in the route's path template.
///
/// ```rust
/// use pavex_builder::{f, router::GET, Blueprint};
/// use pavex_runtime::extract::route::RouteParams;
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // [...]
/// // Register a route with a few route parameters.
/// bp.route(GET, "/address/:address_id/home/:home_id/room/:room_id/", f!(crate::get_home));
/// # }
///
/// #[derive(serde::Deserialize)]
/// struct HomeRouteParams {
///     // The name of the field must match the name of the route parameter
///     // used in the template we passed to `bp.route`.
///     home_id: u32,
///     // You can map a route parameter to a struct field with a different
///     // name via the `rename` attribute.
///     #[serde(rename(deserialize = "address_id"))]
///     street_id: String,
///     // You can also choose to ignore some route parameters—e.g. we are not
///     // extracting the `room_id` here.
/// }
///
/// // The `RouteParams` extractor will deserialize the route parameters into the
/// // type you specified—`HomeRouteParams` in this case.
/// fn get_home(params: &RouteParams<HomeRouteParams>) -> String {
///     let params = &params.0;
///     format!("The home with id {} is in street {}", params.home_id, params.street_id)
/// }
/// ```
///
/// # Unsupported types
///
/// `pavex` wants to enable local reasoning, whenever possible: it should be easy to understand what
/// each extracted route parameter represents.  
/// Struct with named fields are ideal in this regard: by looking at the field name you can
/// immediately understand _which_ route parameter is being extracted.  
/// The same is not true for other types, e.g. `(String, u64, u32)`, where you have to go and
/// check the route's path template to understand what each entry represents.
/// For this reason, `pavex` does not support the following types as `T` in `RouteParams<T>`:
///
/// - tuples, e.g. `(u32, String)`;
/// - tuple structs, e.g. `struct HomeId(u32, String)`;
/// - unit structs, e.g. `struct HomeId`;
/// - newtypes, e.g. `struct HomeId(MyParamsStruct)`;
/// - sequence-like or map-like types, e.g. `Vec<String>` or `HashMap<String, String>`;
/// - enums.
///
/// # Working with raw route parameters
///
/// It is possible to work with the **raw** route parameters, i.e. the route parameters as they
/// are extracted from the URL, before any kind of percent-decoding or deserialization has taken
/// place.
///
/// You can do so by using the [`RawRouteParams`] extractor instead of [`RouteParams`]. Check out
/// [`RawRouteParams`]' documentation for more information.
#[doc(alias = "Path")]
#[doc(alias = "PathParams")]
#[doc(alias = "UrlParams")]
pub struct RouteParams<T>(
    /// The extracted route parameters, deserialized into `T`, the type you specified.
    pub T,
);

impl<T> RouteParams<T> {
    /// The default constructor for [`RouteParams`].
    ///
    /// If the extraction fails, an [`ExtractRouteParamsError`] returned.
    pub fn extract<'key, 'value>(
        params: RawRouteParams<'key, 'value>,
    ) -> Result<Self, ExtractRouteParamsError>
    where
        T: Deserialize<'value>,
        // The parameter ids live as long as the server, while the values are tied to the lifecycle
        // of an incoming request. So it's always true that 'key outlives 'value.
        'key: 'value,
    {
        let mut decoded_params = Vec::with_capacity(params.len());
        for (id, value) in params.iter() {
            let decoded_value = percent_decode_str(value).decode_utf8().map_err(|e| {
                ExtractRouteParamsError::InvalidUtf8InPathParameter(InvalidUtf8InPathParam {
                    invalid_key: id.into(),
                    invalid_raw_segment: value.into(),
                    source: e,
                })
            })?;
            decoded_params.push((id, decoded_value));
        }
        let deserializer = PathDeserializer::new(&decoded_params);
        T::deserialize(deserializer)
            .map_err(ExtractRouteParamsError::PathDeserializationError)
            .map(RouteParams)
    }
}
