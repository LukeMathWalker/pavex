use std::borrow::Cow;

use matchit::{Params, ParamsIter};
use percent_encoding::percent_decode_str;

use crate::request::path::errors::DecodeError;

/// Extract (raw) path parameters from the URL of an incoming request.
///
/// # Guide
///
/// Check out [the guide](https://pavex.dev/docs/guide/request_data/path/raw_path_parameters/)
/// for more information on how to use this extractor.
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
///     // Register a route with a few path parameters.
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
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RawPathParams<'server, 'request>(Params<'server, 'request>);

impl Default for RawPathParams<'_, '_> {
    fn default() -> Self {
        Self(Params::new())
    }
}

impl<'server, 'request> RawPathParams<'server, 'request> {
    /// Returns the number of extracted path parameters.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns the value of the first path parameter registered under the given key.
    pub fn get(&self, key: impl AsRef<str>) -> Option<&'request str> {
        self.0.get(key)
    }

    /// Returns an iterator over the parameters in the list.
    pub fn iter(&self) -> RawPathParamsIter<'_, 'server, 'request> {
        RawPathParamsIter(self.0.iter())
    }

    /// Returns `true` if no path parameters have been extracted from the request's path.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'k, 'v> From<Params<'k, 'v>> for RawPathParams<'k, 'v> {
    fn from(value: Params<'k, 'v>) -> Self {
        Self(value)
    }
}

/// An iterator over the path parameters extracted via [`RawPathParams`].
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

/// A wrapper around a percent-encoded path parameter, obtained via [`RawPathParams`].
///
/// Use [`decode`](Self::decode) to extract the percent-encoded value.
#[derive(Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct EncodedParamValue<'request>(&'request str);

impl<'request> EncodedParamValue<'request> {
    fn new(s: &'request str) -> Self {
        Self(s)
    }

    /// Percent-decode a raw path parameter.
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
