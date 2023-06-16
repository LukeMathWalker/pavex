use matchit::{Params, ParamsIter};

/// Extract (raw) route parameters from the URL of an incoming request.
///
/// # Example
///
/// ```rust
/// use pavex::f;
/// use pavex::blueprint::{router::GET, Blueprint};
/// use pavex::extract::route::RawRouteParams;
///
/// fn blueprint() -> Blueprint {
///     let mut bp = Blueprint::new();
///     // [...]
///     // Register a route with a few route parameters.
///     bp.route(GET, "/address/:address_id/home/:home_id", f!(crate::get_home));
///     bp
/// }
///
/// pub fn get_home(params: &RawRouteParams) -> String {
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
///
/// [`RouteParams`]: struct@crate::extract::route::RouteParams
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RawRouteParams<'server, 'request>(Params<'server, 'request>);

impl<'server, 'request> RawRouteParams<'server, 'request> {
    /// Returns the number of extracted route parameters.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns the value of the first route parameter registered under the given key.
    pub fn get(&self, key: impl AsRef<str>) -> Option<&'request str> {
        self.0.get(key)
    }

    /// Returns an iterator over the parameters in the list.
    pub fn iter(&self) -> RawRouteParamsIter<'_, 'server, 'request> {
        RawRouteParamsIter(self.0.iter())
    }

    /// Returns `true` if no route parameters have been extracted from the request URL.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'k, 'v> From<Params<'k, 'v>> for RawRouteParams<'k, 'v> {
    fn from(value: Params<'k, 'v>) -> Self {
        Self(value)
    }
}

/// An iterator over the route parameters extracted via [`RawRouteParams`].
pub struct RawRouteParamsIter<'extractor, 'server, 'request>(
    ParamsIter<'extractor, 'server, 'request>,
);

impl<'extractor, 'server, 'request> Iterator for RawRouteParamsIter<'extractor, 'server, 'request> {
    type Item = (&'server str, &'request str);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}
