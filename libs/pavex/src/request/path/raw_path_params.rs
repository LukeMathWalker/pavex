use std::borrow::Cow;

use matchit::{Params, ParamsIter};
use percent_encoding::percent_decode_str;

use crate::request::path::errors::DecodeError;

/// Extract (raw) route parameters from the URL of an incoming request.
///
/// # Example
///
/// ```rust
/// use pavex::f;
/// use pavex::blueprint::{router::GET, Blueprint};
/// use pavex::request::path::RawPathParams;
///
/// fn blueprint() -> Blueprint {
///     let mut bp = Blueprint::new();
///     // [...]
///     // Register a route with a few route parameters.
///     bp.route(GET, "/address/:address_id/home/:home_id", f!(crate::get_home));
///     bp
/// }
///
/// pub fn get_home(params: &RawPathParams) -> String {
///     let home_id = &params.get("home_id").unwrap();
///     let street_id = &params.get("street_id").unwrap();
///     format!("The home with id {} is in street {}", home_id, street_id)
/// }
/// ```
///
/// # Framework primitive
///
/// `RawPathParams` is a framework primitive—you don't need to register any constructor
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
/// `RawPathParams` gives you access to the **raw** route parameters, i.e. the route parameters
/// as they are extracted from the URL, before any kind of processing has taken
/// place.
///
/// In particular, `RawPathParams` does **not** perform any percent-decoding.  
/// If you send a request to `/address/123%20456/home/789`, the `RawPathParams` for
/// `/address/:address_id/home/:home_id` will contain the following key-value pairs:
///
/// - `address_id`: `123%20456`
/// - `home_id`: `789`
///
/// `address_id` is not `123 456` because `RawPathParams` does not perform percent-decoding!
/// Therefore `%20` is not interpreted as a space character.
///
/// There are situations where you might want to work with the raw route parameters, but
/// most of the time you'll want to use [`PathParams`] instead—it performs percent-decoding
/// and deserialization for you.
///
/// [`PathParams`]: struct@crate::request::path::PathParams
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RawPathParams<'server, 'request>(Params<'server, 'request>);

impl<'server, 'request> RawPathParams<'server, 'request> {
    /// Returns the number of extracted route parameters.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns the value of the first route parameter registered under the given key.
    pub fn get(&self, key: impl AsRef<str>) -> Option<&'request str> {
        self.0.get(key)
    }

    /// Returns an iterator over the parameters in the list.
    pub fn iter(&self) -> RawPathParamsIter<'_, 'server, 'request> {
        RawPathParamsIter(self.0.iter())
    }

    /// Returns `true` if no route parameters have been extracted from the request URL.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'k, 'v> From<Params<'k, 'v>> for RawPathParams<'k, 'v> {
    fn from(value: Params<'k, 'v>) -> Self {
        Self(value)
    }
}

/// An iterator over the route parameters extracted via [`RawPathParams`].
pub struct RawPathParamsIter<'extractor, 'server, 'request>(
    ParamsIter<'extractor, 'server, 'request>,
);

impl<'extractor, 'server, 'request> Iterator for RawPathParamsIter<'extractor, 'server, 'request> {
    type Item = (&'server str, EncodedParamValue<'request>);

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            .map(|(key, value)| (key, EncodedParamValue::new(value)))
    }
}

/// A wrapper around a percent-encoded route parameter, obtained via [`RawPathParams`].
///
/// Use [`decode`](Self::decode) to extract the percent-encoded value.
#[derive(Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct EncodedParamValue<'request>(&'request str);

impl<'request> EncodedParamValue<'request> {
    fn new(s: &'request str) -> Self {
        Self(s)
    }

    /// Percent-decode a raw route parameter.
    ///
    /// If decoding fails, a [`DecodeError`] is returned.
    pub fn decode(&self) -> Result<Cow<'request, str>, DecodeError> {
        percent_decode_str(self.0)
            .decode_utf8()
            .map_err(|e| DecodeError {
                invalid_raw_segment: self.0.to_owned(),
                source: e,
            })
    }

    /// Get a reference to the underlying percent-encoded string.
    pub fn as_str(&self) -> &'request str {
        self.0
    }
}
