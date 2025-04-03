use crate::request::RequestHead;
use crate::request::body::BufferedBody;
use crate::request::body::errors::{
    ExtractUrlEncodedBodyError, MissingUrlEncodedContentType, UrlEncodedBodyDeserializationError,
    UrlEncodedContentTypeMismatch,
};
use http::HeaderMap;
use pavex_macros::request_scoped;
use serde::Deserialize;

#[doc(alias = "UrlEncoded")]
#[doc(alias = "Form")]
#[doc(alias = "FormBody")]
#[doc(alias = "PercentEncoded")]
#[doc(alias = "PercentEncodedBody")]
#[derive(Debug)]
/// Parse a URL-encoded request body, such as a web form.
///
/// # Guide
///
/// Check out the [relevant section](https://pavex.dev/docs/guide/request_data/body/deserializers/url_encoded/)
/// of Pavex's guide for a thorough introduction to `UrlEncodedBody`.
///
/// # Example
///
/// ```rust
/// use pavex::request::body::UrlEncodedBody;
///
/// // You must derive `serde::Deserialize` for the type you want to extract,
/// // in this case `HomeListing`.
/// #[derive(serde::Deserialize)]
/// pub struct HomeListing {
///     address: String,
///     price: u64,
/// }
///
/// // The `UrlEncodedBody` extractor deserializes the request body into
/// // the type you specified—`HomeListing` in this case.
/// pub fn get_home(body: &UrlEncodedBody<HomeListing>) -> String {
///     format!(
///         "The home you want to sell for {} is located at {}",
///         body.0.price,
///         body.0.address
///     )
/// }
/// ```
pub struct UrlEncodedBody<T>(pub T);

impl<T> UrlEncodedBody<T> {
    #[request_scoped(
        error_handler = "crate::request::body::errors::ExtractUrlEncodedBodyError::into_response"
    )]
    pub fn extract<'head, 'body>(
        request_head: &'head RequestHead,
        buffered_body: &'body BufferedBody,
    ) -> Result<Self, ExtractUrlEncodedBodyError>
    where
        T: Deserialize<'body>,
    {
        check_urlencoded_content_type(&request_head.headers)?;
        parse(buffered_body.bytes.as_ref()).map(UrlEncodedBody)
    }
}

/// Check that the `Content-Type` header is set to `application/x-www-form-urlencoded`.
///
/// Return an error otherwise.
fn check_urlencoded_content_type(headers: &HeaderMap) -> Result<(), ExtractUrlEncodedBodyError> {
    let Some(content_type) = headers.get(http::header::CONTENT_TYPE) else {
        return Err(MissingUrlEncodedContentType.into());
    };
    let Ok(content_type) = content_type.to_str() else {
        return Err(MissingUrlEncodedContentType.into());
    };

    let Ok(mime) = content_type.parse::<mime::Mime>() else {
        return Err(UrlEncodedContentTypeMismatch {
            actual: content_type.to_string(),
        }
        .into());
    };

    let is_urlencoded_content_type =
        mime.type_() == mime::APPLICATION && mime.subtype() == mime::WWW_FORM_URLENCODED;
    if !is_urlencoded_content_type {
        return Err(UrlEncodedContentTypeMismatch {
            actual: content_type.to_string(),
        }
        .into());
    };
    Ok(())
}

/// Parse bytes into `T`.
fn parse<'a, T>(bytes: &'a [u8]) -> Result<T, ExtractUrlEncodedBodyError>
where
    T: Deserialize<'a>,
{
    serde_html_form::from_bytes(bytes)
        .map_err(|e| UrlEncodedBodyDeserializationError { source: e })
        .map_err(ExtractUrlEncodedBodyError::DeserializationError)
}

#[cfg(test)]
mod tests {
    use crate::request::body::UrlEncodedBody;
    use std::borrow::Cow;

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
        let actual: Home = crate::request::body::url_encoded::parse(query.as_bytes()).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn missing_content_type() {
        let headers = http::HeaderMap::new();
        let err = super::check_urlencoded_content_type(&headers).unwrap_err();
        insta::assert_snapshot!(err, @"The `Content-Type` header is missing. This endpoint expects requests with a `Content-Type` header set to `application/x-www-form-urlencoded`");
        insta::assert_debug_snapshot!(err, @r###"
        MissingContentType(
            MissingUrlEncodedContentType,
        )
        "###);
    }

    #[test]
    fn content_type_is_not_valid_mime() {
        let mut headers = http::HeaderMap::new();
        headers.insert(http::header::CONTENT_TYPE, "hello world".parse().unwrap());

        let err = super::check_urlencoded_content_type(&headers).unwrap_err();
        insta::assert_snapshot!(err, @"The `Content-Type` header was set to `hello world`. This endpoint expects requests with a `Content-Type` header set to `application/x-www-form-urlencoded`");
        insta::assert_debug_snapshot!(err, @r###"
        ContentTypeMismatch(
            UrlEncodedContentTypeMismatch {
                actual: "hello world",
            },
        )
        "###);
    }

    #[test]
    fn content_type_is_not_form() {
        let mut headers = http::HeaderMap::new();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );

        let err = super::check_urlencoded_content_type(&headers).unwrap_err();
        insta::assert_snapshot!(err, @"The `Content-Type` header was set to `application/json`. This endpoint expects requests with a `Content-Type` header set to `application/x-www-form-urlencoded`");
        insta::assert_debug_snapshot!(err, @r###"
        ContentTypeMismatch(
            UrlEncodedContentTypeMismatch {
                actual: "application/json",
            },
        )
        "###);
    }

    #[test]
    fn content_type_is_form() {
        let mut headers = http::HeaderMap::new();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded".parse().unwrap(),
        );

        let outcome = super::check_urlencoded_content_type(&headers);
        assert!(outcome.is_ok());
    }

    #[test]
    fn form_content_type_with_charset() {
        let mut headers = http::HeaderMap::new();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded; charset=utf-8"
                .parse()
                .unwrap(),
        );

        let outcome = super::check_urlencoded_content_type(&headers);
        assert!(outcome.is_ok());
    }

    #[test]
    /// Let's check the error quality when the request body is missing
    /// a required field.
    fn missing_form_field() {
        // Arrange
        #[derive(serde::Deserialize, Debug)]
        #[allow(dead_code)]
        struct BodySchema {
            name: String,
            surname: String,
            age: u8,
        }

        let mut headers = http::HeaderMap::new();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded".parse().unwrap(),
        );
        let request_head = crate::request::RequestHead {
            headers,
            method: http::Method::POST,
            version: http::Version::HTTP_11,
            target: "/".parse().unwrap(),
        };
        let body = "name=John%20Doe&age=43".to_string();

        // Act
        let buffered_body = crate::request::body::BufferedBody { bytes: body.into() };
        let outcome: Result<UrlEncodedBody<BodySchema>, _> =
            UrlEncodedBody::extract(&request_head, &buffered_body);

        // Assert
        let err = outcome.unwrap_err();
        insta::assert_snapshot!(err, @r###"
        Failed to deserialize the body as a urlencoded form.
        missing field `surname`
        "###);
        insta::assert_debug_snapshot!(err, @r###"
        DeserializationError(
            UrlEncodedBodyDeserializationError {
                source: Error(
                    "missing field `surname`",
                ),
            },
        )
        "###);
    }
}
