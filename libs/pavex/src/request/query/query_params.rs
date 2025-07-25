use pavex_macros::methods;

use crate::request::RequestHead;

use super::errors::{ExtractQueryParamsError, QueryDeserializationError};

/// Extract (typed) query parameters from the query of an incoming request.
///
/// # Guide
///
/// Check out [the guide](https://pavex.dev/docs/guide/request_data/query/query_parameters/)
/// for more information on how to use this extractor.
///
/// # Example
///
/// ```rust
/// use pavex::request::query::QueryParams;
/// // You must derive `serde::Deserialize` for the type you want to extract,
/// // in this case `Home`.
/// #[derive(serde::Deserialize)]
/// pub struct Home {
///     home_id: u32
/// }
///
/// // The `QueryParams` extractor deserializes the extracted query parameters into
/// // the type you specified—`Home` in this case.
/// pub fn get_home(params: &QueryParams<Home>) -> String {
///    format!("The identifier for this home is: {}", params.0.home_id)
/// }
/// ```
///
/// The `home_id` field will be set to `1` for the `?home_id=1` query string.
///
#[doc(alias = "Query")]
#[doc(alias = "QueryParameters")]
pub struct QueryParams<T>(
    /// The extracted query parameters, deserialized into `T`, the type you specified.
    pub T,
);

#[methods]
impl<T> QueryParams<T> {
    /// The default constructor for [`QueryParams`].
    ///
    /// If the extraction fails, an [`ExtractQueryParamsError`] is returned.
    ///
    /// Check out [`QueryParams`] for more information on query parameters.
    #[request_scoped(pavex = crate, id = "QUERY_PARAMS_EXTRACT")]
    pub fn extract<'request>(
        request_head: &'request RequestHead,
    ) -> Result<Self, ExtractQueryParamsError>
    where
        T: serde::Deserialize<'request>,
    {
        let query = request_head.target.query().unwrap_or_default();
        parse(query).map(QueryParams)
    }
}

/// Parse a query string into a `T`.
fn parse<'a, T>(s: &'a str) -> Result<T, ExtractQueryParamsError>
where
    T: serde::Deserialize<'a>,
{
    let deserializer = serde_html_form::Deserializer::new(form_urlencoded::parse(s.as_bytes()));
    serde_path_to_error::deserialize(deserializer)
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
