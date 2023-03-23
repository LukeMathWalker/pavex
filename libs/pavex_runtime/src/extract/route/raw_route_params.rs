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
