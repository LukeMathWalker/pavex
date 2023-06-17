use percent_encoding::percent_decode_str;
use serde::Deserialize;

use crate::extract::route::deserializer::PathDeserializer;
use crate::extract::route::errors::{ExtractRouteParamsError, InvalidUtf8InPathParam};

use super::RawRouteParams;

/// Extract (typed) route parameters from the URL of an incoming request.
///
/// # Sections
///
/// - [Example](#example)
/// - [Supported types](#supported-types)
/// - [Unsupported types](#unsupported-types)
/// - [Additional compile-time checks](#additional-compile-time-checks)
/// - [Avoiding allocations](#avoiding-allocations)
/// - [Working with raw route parameters](#working-with-raw-route-parameters)
///
/// # Example
///
/// ```rust
/// use pavex::f;
/// use pavex::blueprint::{router::GET, Blueprint, constructor::Lifecycle};
/// use pavex::extract::route::RouteParams;
///
/// fn blueprint() -> Blueprint {
///     let mut bp = Blueprint::new();
///     // Register the default constructor and error handler for `RouteParams`.
///     bp.constructor(
///         f!(pavex::extract::route::RouteParams::extract),
///         Lifecycle::RequestScoped,
///     ).error_handler(
///         f!(pavex::extract::route::errors::ExtractRouteParamsError::into_response)
///     );
///     // Register a route with a route parameter, `:home_id`.
///     bp.route(GET, "/home/:home_id", f!(crate::get_home));
///     bp
/// }
///
/// // The RouteParams attribute macro derives the necessary (de)serialization traits.
/// #[RouteParams]
/// pub struct Home {
///     // The name of the field must match the name of the route parameter
///     // used in `bp.route`.
///     home_id: u32
/// }
///
/// // The `RouteParams` extractor deserializes the extracted route parameters into
/// // the type you specified—`HomeRouteParams` in this case.
/// pub fn get_home(params: &RouteParams<Home>) -> String {
///    format!("The identifier for this home is: {}", params.0.home_id)
/// }
/// ```
///
/// `home_id` will be set to `1` for an incoming `/home/1` request.  
/// Extraction will fail, instead, if we receive an `/home/abc` request.
///
/// # Supported types
///
/// `T` in `RouteParams<T>` must implement [`serde::Deserialize`]—it is automatically derived if
/// you use the [`RouteParams`](macro@crate::extract::route::RouteParams) attribute macro, the
/// approach we recommend.  
/// `T` must be a struct with named fields, where each field name matches one of the route parameter
/// names used in the route's path template.
///
/// ```rust
/// use pavex::f;
/// use pavex::blueprint::{router::GET, Blueprint};
/// use pavex::extract::route::RouteParams;
///
/// fn blueprint() -> Blueprint{
///     let mut bp = Blueprint::new();
///     // [...]
///     // Register a route with a few route parameters.
///     bp.route(GET, "/address/:address_id/home/:home_id/room/:room_id/", f!(crate::get_room));
///     bp
/// }
///
/// #[RouteParams]
/// pub struct Room {
///     // The name of the extracted fields must match the names of the route parameters
///     // used in the template we passed to `bp.route`.
///     home_id: u32,
///     street_id: String,
///     // You can also choose to ignore some route parameters—e.g. we are not
///     // extracting the `room_id` here.
/// }
///
/// // The `RouteParams` extractor will deserialize the route parameters into the
/// // type you specified—`Room` in this case.
/// pub fn get_room(params: &RouteParams<Room>) -> String {
///     let params = &params.0;
///     format!("The home with id {} is in street {}", params.home_id, params.street_id)
/// }
/// ```
///
/// # Unsupported types
///
/// Pavex wants to enable local reasoning: it should be easy to understand what
/// each extracted route parameter represents.  
/// Plain structs with named fields are ideal in this regard: by looking at the field name you can
/// immediately understand _which_ route parameter is being extracted.  
/// The same is not true for other types, e.g. `(String, u64, u32)`, where you have to go and
/// check the route's path template to understand what each entry represents.
///
///```rust
/// use pavex::extract::route::RouteParams;
///
/// // This is self-documenting ✅
/// // No need to check the route's path template to understand what each field represents.
/// #[RouteParams]
/// pub struct Room {
///     home_id: u32,
///     room_id: u32,
///     street_id: u32,
/// }
///
/// pub fn get_room(params: &RouteParams<Room>) -> String {
///     // [...]
/// # unimplemented!()
/// }
///
/// // This isn't self-documenting ❌
/// // What does the second u32 represent? The room id? The street id?
/// // Impossible to tell without checking the route's path template.
/// pub fn get_room_tuple(params: &RouteParams<(u32, u32, u32)>) -> String {
///     // [...]
/// # unimplemented!()
/// }
/// ```
///
/// For this reason, Pavex does not support the following types as `T` in `RouteParams<T>`:
///
/// - tuples, e.g. `(u32, String)`;
/// - tuple structs, e.g. `struct HomeId(u32, String)`;
/// - unit structs, e.g. `struct HomeId`;
/// - newtypes, e.g. `struct HomeId(MyParamsStruct)`;
/// - sequence-like or map-like types, e.g. `Vec<String>` or `HashMap<String, String>`;
/// - enums.
///
/// # Additional compile-time checks
///
/// Pavex is able to perform additional checks at compile-time if you use the
/// [`RouteParams`](macro@crate::extract::route::RouteParams) macro instead
/// of deriving [`serde::Deserialize`] on your own.
///
/// ```rust
/// # mod home {
/// use pavex::extract::route::RouteParams;
///
/// // Do this 👇
/// #[RouteParams]
/// pub struct Home {
///     home_id: u32
/// }
/// # }
///
/// # mod home2 {
/// // ..instead of this ❌
/// #[derive(serde::Deserialize)]
/// pub struct Home {
///     home_id: u32
/// }
/// # }
/// ```
///
/// In particular, Pavex becomes able to:
///
/// - verify that for each field in the struct there is a corresponding route parameter
///   in the route's path.
/// - detect the usage of common unsupported types as fields, e.g. vectors, tuples.
/// - detect common errors that might result in a runtime error, e.g. using `&str` as a field type
///   instead of `Cow<'_, str>` (see [`Avoiding allocations`](#avoiding-allocations)).
///
/// Check out [`StructuralDeserialize`](crate::serialization::StructuralDeserialize) if you are curious
/// to know more about the role played by the [`RouteParams`](macro@crate::extract::route::RouteParams)
/// macro in enabling these additional compile-time checks.
///
/// # Avoiding allocations
///
/// If you want to squeeze out the last bit of performance from your application, you can try to
/// avoid memory allocations when extracting string-like route parameters.  
/// Pavex supports this use case—you can borrow from the request's URL instead of cloning.
///
/// It is not always possible to avoid allocations, though.  
/// In particular, if the route parameter is a URL-encoded string (e.g. `John%20Doe`, the URL-encoded
/// version of `John Doe`) Pavex must allocate a new `String` to store the decoded version.
///
/// Given the above, we recommend using `Cow<'_, str>` as field type: it borrows from the request's
/// URL if possible, and allocates a new `String` only if strictly necessary.
///
/// ```rust
/// use pavex::extract::route::RouteParams;
/// use std::borrow::Cow;
///
/// #[RouteParams]
/// pub struct Payee<'a> {
///     name: Cow<'a, str>,
/// }
///
/// pub fn get_payee(params: &RouteParams<Payee<'_>>) -> String {
///    format!("The payee's name is {}", params.0.name)
/// }
/// ```
///
/// Using `&str` instead of `Cow<'_, str>` would result in a runtime error if the route parameter
/// is URL-encoded. It is therefore discouraged and Pavex will emit an error at compile-time
/// if it detects this pattern.
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
    /// If the extraction fails, an [`ExtractRouteParamsError`] is returned.
    pub fn extract<'server, 'request>(
        params: RawRouteParams<'server, 'request>,
    ) -> Result<Self, ExtractRouteParamsError>
    where
        T: Deserialize<'request>,
        // The parameter ids live as long as the server, while the values are tied to the lifecycle
        // of an incoming request. So it's always true that 'key outlives 'value.
        'server: 'request,
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
