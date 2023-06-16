use super::errors::{ExtractQueryParamsError, QueryDeserializationError};
use crate::request::RequestHead;

/// Extract (typed) route parameters from the query parameters of an incoming request.
///
/// # Sections
///
/// - [Example](#example)
/// - [Installation](#installtion)
/// - [Supported types](#supported-types)
///   - [Sequences](#sequences)
/// - [Unsupported types](#unsupported-types)
/// - [Avoiding allocations](#avoiding-allocations)
///
/// # Example
///
/// ```rust
/// use pavex::extract::query::QueryParams;
/// // You must derive `serde::Deserialize` for the type you want to extract,
/// // in this case `Home`.
/// #[derive(serde::Deserialize)]
/// pub struct Home {
///     home_id: u32
/// }
///
/// // The `RouteParams` extractor deserializes the extracted route parameters into
/// // the type you specified—`HomeRouteParams` in this case.
/// pub fn get_home(params: &QueryParams<Home>) -> String {
///    format!("The identifier for this home is: {}", params.0.home_id)
/// }
/// ```
///
/// The `home_id` field will be set to `1` for the `?home_id=1` query string.
///
/// ## Installation
///
/// First of all, you need the register the default constructor and error handler for
/// `QueryParams` in your `Blueprint`:
///
/// ```rust
/// use pavex::f;
/// use pavex::blueprint::{Blueprint, constructor::Lifecycle};
///
/// fn blueprint() -> Blueprint {
///     let mut bp = Blueprint::new();
///     // Register the default constructor and error handler for `QueryParams`.
///     bp.constructor(
///         f!(pavex::extract::query::QueryParams::extract),
///         Lifecycle::RequestScoped,
///     ).error_handler(
///         f!(pavex::extract::query::errors::ExtractQueryParamsError::into_response)
///     );
///     // [...]
///     bp
/// }
/// ```
///
/// You can then use the `QueryParams` extractor as input to your route handlers and constructors.
///
/// # Supported types
///
/// `T` in `QueryParams<T>` must implement [`serde::Deserialize`].  
/// You can derive this trait automatically by applying `#[derive(serde::Deserialize)]`
/// to your type.
///
/// ## Sequences
///
/// There is no standard way to represent sequences in query parameters.  
/// Pavex supports the [form style](https://swagger.io/docs/specification/serialization/#query), as
/// specified by OpenAPI:
///
/// ```rust
/// use pavex::extract::query::QueryParams;
///
/// #[derive(serde::Deserialize)]
/// pub struct Home {
///    // This will convert the query string `?room_id=1&room_id=2&room_id=3`
///    // into a vector `vec![1, 2, 3]`.  
///    //
///    // Pavex does not perform any pluralization, therefore you must use
///    // `serde`'s rename attribute if you want to use a pluralized name
///    // as struct field but a singularized name in the query string.
///    #[serde(rename = "room_id")]
///    room_ids: Vec<u32>
/// }
/// ```
///
/// Another common way to represent sequences in query parameters is to use brackets.
/// E.g. `?room_ids[]=1&room_ids[]=2&room_ids[]=3`.
///
/// You can use the `serde`'s rename attribute to support this style:
///
/// ```rust
/// use pavex::extract::query::QueryParams;
///
/// #[derive(serde::Deserialize)]
/// pub struct Home {
///     // This will convert the query string `?room_id[]=1&room_id[]=2&room_id[]=3`
///     // into a vector `vec![1, 2, 3]`.
///     #[serde(rename = "room_id[]")]
///     room_ids: Vec<u32>
/// }
/// ```
///
/// # Unsupported types
///
/// Pavex does not support the following types as `T` in `RouteParams<T>`:
///
/// - tuples, e.g. `(u32, String)`;
/// - tuple structs, e.g. `struct HomeId(u32, String)`;
/// - unit structs, e.g. `struct HomeId`;
///
/// You should always prefer a struct with named fields as the type parameter of `QueryParams`.
///
/// When it comes to structs, it's important to keep in mind that `QueryParams` doesn't
/// support deserializing **nested** structures as query parameters.  
/// For example, the following can't be deserialized from the wire using `QueryParams`:
///
/// ```rust
/// use pavex::extract::query::QueryParams;
///
/// #[derive(serde::Deserialize)]
/// pub struct Home {
///    address: Address
/// }
///
/// #[derive(serde::Deserialize)]
/// pub struct Address {
///    street: String,
///    city: String,
/// }
/// ```
///
/// If you need to deserialize nested structures from query parameters, you might want to
/// look into writing your own extractor on top of [`serde_qs`](https://crates.io/crates/serde_qs).
///
/// # Avoiding allocations
///
/// If you want to minimize memory usage, you can try to avoid unnecessary memory allocations when
/// deserializing string-like fields from the query parameters of the incoming request.    
/// Pavex supports this use case—you can borrow from the query string instead of having to
/// allocate a brand new string.
///
/// It is not always possible to avoid allocations, though.  
/// In particular, Pavex *must* allocate a new `String` if the parameter you are trying to
/// deserialize is a URL-encoded string (e.g. `John%20Doe`, the URL-encoded
/// version of `John Doe`)
/// Using a `&str` in this case would result in a runtime error when attempting the deserialization.
///
/// Given the above, we recommend using `Cow<'_, str>` as field type: it borrows from the request
/// body if possible, and allocates a new `String` only if strictly necessary.
///
/// ```rust
/// use pavex::extract::query::QueryParams;
/// use std::borrow::Cow;
///
/// #[derive(serde::Deserialize)]
/// pub struct Payee<'a> {
///     name: Cow<'a, str>,
/// }
///
/// pub fn get_payee(params: &QueryParams<Payee<'_>>) -> String {
///    format!("The payee's name is {}", params.0.name)
/// }
/// ```
#[doc(alias = "Query")]
pub struct QueryParams<T>(
    /// The extracted query parameters, deserialized into `T`, the type you specified.
    pub T,
);

impl<T> QueryParams<T> {
    /// The default constructor for [`QueryParams`].
    ///
    /// If the extraction fails, an [`ExtractQueryParamsError`] is returned.
    ///
    /// Check out [`QueryParams`] for more information on query parameters.
    pub fn extract<'request>(
        request_head: &'request RequestHead,
    ) -> Result<Self, ExtractQueryParamsError>
    where
        T: serde::Deserialize<'request>,
    {
        let query = request_head.uri.query().unwrap_or_default();
        parse(query).map(QueryParams)
    }
}

/// Parse a query string into a `T`.
fn parse<'a, T>(s: &'a str) -> Result<T, ExtractQueryParamsError>
where
    T: serde::Deserialize<'a>,
{
    serde_html_form::from_str(s)
        .map_err(QueryDeserializationError::new)
        .map_err(ExtractQueryParamsError::QueryDeserializationError)
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;

    #[test]
    fn test_parse() {
        #[derive(serde::Deserialize, Debug, PartialEq)]
        struct Home<'a> {
            home_id: u32,
            home_price: f64,
            home_name: Cow<'a, str>,
        }

        let query = "home_id=1&home_price=0.1&home_name=Hi%20there";
        let expected = Home {
            home_id: 1,
            home_price: 0.1,
            home_name: Cow::Borrowed("Hi there"),
        };
        let actual: Home = parse(query).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_sequence() {
        #[derive(serde::Deserialize, Debug, PartialEq)]
        struct Home {
            room_ids: Vec<u32>,
        }

        let query = "room_ids=1&room_ids=2";
        let expected = Home {
            room_ids: vec![1, 2],
        };
        let actual: Home = parse(query).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_sequence_with_brackets() {
        #[derive(serde::Deserialize, Debug, PartialEq)]
        struct Home {
            #[serde(rename = "room_ids[]")]
            room_ids: Vec<u32>,
        }

        let query = "room_ids[]=1&room_ids[]=2";
        let expected = Home {
            room_ids: vec![1, 2],
        };
        let actual: Home = parse(query).unwrap();
        assert_eq!(expected, actual);
    }
}
