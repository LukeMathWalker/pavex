use http::HeaderMap;
use serde::Deserialize;

use crate::blueprint::constructor::{Constructor, Lifecycle, RegisteredConstructor};
use crate::blueprint::Blueprint;
use crate::f;
use crate::request::RequestHead;

use super::{
    buffered_body::BufferedBody,
    errors::{
        ExtractJsonBodyError, JsonContentTypeMismatch, JsonDeserializationError,
        MissingJsonContentType,
    },
};

#[doc(alias = "Json")]
#[derive(Debug)]
/// Parse the body of an incoming request as JSON.
///
/// # Guide
///
/// Check out the [relevant section](https://pavex.dev/docs/guide/request_data/body/deserializers/json/)
/// of Pavex's guide for a thorough introduction to `JsonBody`.
///
/// # Example
///
/// ```rust
/// use pavex::request::body::JsonBody;
///
/// // You must derive `serde::Deserialize` for the type you want to extract,
/// // in this case `HomeListing`.
/// #[derive(serde::Deserialize)]
/// pub struct HomeListing {
///     address: String,
///     price: u64,
/// }
///
/// // The `Json` extractor deserializes the request body into
/// // the type you specified—`HomeListing` in this case.
/// pub fn get_home(body: &JsonBody<HomeListing>) -> String {
///     format!(
///         "The home you want to sell for {} is located at {}",
///         body.0.price,
///         body.0.address
///     )
/// }
/// ```
pub struct JsonBody<T>(pub T);

impl<T> JsonBody<T> {
    /// The default constructor for [`JsonBody`].
    ///
    /// The extraction can fail for a number of reasons:
    ///
    /// - the `Content-Type` is missing
    /// - the `Content-Type` header is not set to `application/json` or another `application/*+json` MIME type
    /// - the request body is not a valid JSON document
    ///
    /// In all of the above cases, an [`ExtractJsonBodyError`] is returned.
    // # Implementation notes
    //
    // We are using two separate lifetimes here to make it clear to the compiler
    // that `JsonBody` doesn't borrow from `RequestHead`.
    pub fn extract<'head, 'body>(
        request_head: &'head RequestHead,
        buffered_body: &'body BufferedBody,
    ) -> Result<Self, ExtractJsonBodyError>
    where
        T: Deserialize<'body>,
    {
        check_json_content_type(&request_head.headers)?;
        let mut deserializer = serde_json::Deserializer::from_slice(buffered_body.bytes.as_ref());
        let body = serde_path_to_error::deserialize(&mut deserializer)
            .map_err(|e| JsonDeserializationError { source: e })?;
        Ok(JsonBody(body))
    }
}

impl JsonBody<()> {
    /// Register the [default constructor](Self::default_constructor)
    /// for [`JsonBody`] with a [`Blueprint`].
    pub fn register(bp: &mut Blueprint) -> RegisteredConstructor {
        Self::default_constructor().register(bp)
    }

    /// The [default constructor](JsonBody::extract)
    /// and [error handler](ExtractJsonBodyError::into_response) for [`JsonBody`].
    pub fn default_constructor() -> Constructor {
        Constructor::new(f!(super::JsonBody::extract), Lifecycle::RequestScoped)
            .error_handler(f!(super::errors::ExtractJsonBodyError::into_response))
    }
}

/// Check that the `Content-Type` header is set to `application/json`, or another
/// `application/*+json` MIME type.
///
/// Return an error otherwise.
fn check_json_content_type(headers: &HeaderMap) -> Result<(), ExtractJsonBodyError> {
    let Some(content_type) = headers.get(http::header::CONTENT_TYPE) else {
        return Err(MissingJsonContentType.into());
    };
    let Ok(content_type) = content_type.to_str() else {
        return Err(MissingJsonContentType.into());
    };

    let Ok(mime) = content_type.parse::<mime::Mime>() else {
        return Err(JsonContentTypeMismatch {
            actual: content_type.to_string(),
        }
        .into());
    };

    let is_json_content_type = mime.type_() == "application"
        && (mime.subtype() == "json" || mime.suffix().map_or(false, |name| name == "json"));
    if !is_json_content_type {
        return Err(JsonContentTypeMismatch {
            actual: content_type.to_string(),
        }
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::request::body::JsonBody;

    #[test]
    fn missing_content_type() {
        let headers = http::HeaderMap::new();
        let err = super::check_json_content_type(&headers).unwrap_err();
        insta::assert_display_snapshot!(err, @"The `Content-Type` header is missing. This endpoint expects requests with a `Content-Type` header set to `application/json`, or another `application/*+json` MIME type");
        insta::assert_debug_snapshot!(err, @r###"
        MissingContentType(
            MissingJsonContentType,
        )
        "###);
    }

    #[test]
    fn content_type_is_not_valid_mime() {
        let mut headers = http::HeaderMap::new();
        headers.insert(http::header::CONTENT_TYPE, "hello world".parse().unwrap());

        let err = super::check_json_content_type(&headers).unwrap_err();
        insta::assert_display_snapshot!(err, @"The `Content-Type` header was set to `hello world`. This endpoint expects requests with a `Content-Type` header set to `application/json`, or another `application/*+json` MIME type");
        insta::assert_debug_snapshot!(err, @r###"
        ContentTypeMismatch(
            JsonContentTypeMismatch {
                actual: "hello world",
            },
        )
        "###);
    }

    #[test]
    fn content_type_is_not_json() {
        let mut headers = http::HeaderMap::new();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/xml".parse().unwrap(),
        );

        let err = super::check_json_content_type(&headers).unwrap_err();
        insta::assert_display_snapshot!(err, @"The `Content-Type` header was set to `application/xml`. This endpoint expects requests with a `Content-Type` header set to `application/json`, or another `application/*+json` MIME type");
        insta::assert_debug_snapshot!(err, @r###"
        ContentTypeMismatch(
            JsonContentTypeMismatch {
                actual: "application/xml",
            },
        )
        "###);
    }

    #[test]
    fn content_type_is_json() {
        let mut headers = http::HeaderMap::new();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );

        let outcome = super::check_json_content_type(&headers);
        assert!(outcome.is_ok());
    }

    #[test]
    fn content_type_has_json_suffix() {
        let mut headers = http::HeaderMap::new();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/hal+json".parse().unwrap(),
        );

        let outcome = super::check_json_content_type(&headers);
        assert!(outcome.is_ok());
    }

    #[test]
    fn json_content_type_with_charset() {
        let mut headers = http::HeaderMap::new();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/json; charset=utf-8".parse().unwrap(),
        );

        let outcome = super::check_json_content_type(&headers);
        assert!(outcome.is_ok());
    }

    #[test]
    /// Let's check the error quality when the request body is missing
    /// a required field.
    fn missing_json_field() {
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
            "application/json; charset=utf-8".parse().unwrap(),
        );
        let request_head = crate::request::RequestHead {
            headers,
            method: http::Method::GET,
            target: "/".parse().unwrap(),
            version: http::Version::HTTP_11,
        };
        let body = serde_json::json!({
            "name": "John Doe",
            "age": 43,
        });

        // Act
        let buffered_body = crate::request::body::BufferedBody {
            bytes: serde_json::to_vec(&body).unwrap().into(),
        };
        let outcome: Result<JsonBody<BodySchema>, _> =
            JsonBody::extract(&request_head, &buffered_body);

        // Assert
        let err = outcome.unwrap_err();
        insta::assert_display_snapshot!(err, @r###"
        Failed to deserialize the body as a JSON document.
        missing field `surname` at line 1 column 28
        "###);
        insta::assert_debug_snapshot!(err, @r###"
        DeserializationError(
            JsonDeserializationError {
                source: Error {
                    path: Path {
                        segments: [],
                    },
                    original: Error("missing field `surname`", line: 1, column: 28),
                },
            },
        )
        "###);
    }
}
