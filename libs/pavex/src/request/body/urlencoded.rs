use crate::request::body::errors::{
    ExtractUrlEncodedBodyError, MissingUrlEncodedContentType, UrlEncodedBodyDeserializationError,
    UrlEncodedContentTypeMismatch, UrlEncodedQueryDeserializationError,
};
use crate::request::body::BufferedBody;
use crate::request::RequestHead;
use http::{HeaderMap, Method};
use serde::Deserialize;

#[doc(alias = "UrlEncoded")]
#[derive(Debug)]
/// Parse the body of an incoming request as an application/x-www-form-urlencoded form.
///
/// # Sections
///
/// - [Example](#example)
/// - [Installation](#installtion)
/// - [Avoiding allocations](#avoiding-allocations)
/// - [Body size limit](#body-size-limit)
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
///
/// # Installation
///
/// First of all, you need the register the default constructor and error handler for
/// `UrlEncodedBody` in your `Blueprint`:
///
/// ```rust
/// use pavex::f;
/// use pavex::blueprint::{Blueprint, constructor::Lifecycle};
///
/// fn blueprint() -> Blueprint {
///    let mut bp = Blueprint::new();
///    // Register the default constructor and error handler for `UrlEncodedBody`.
///    bp.constructor(
///         f!(pavex::request::body::UrlEncodedBody::extract),
///         Lifecycle::RequestScoped,
///     ).error_handler(
///         f!(pavex::request::body::errors::ExtractUrlEncodedBodyError::into_response)
///     );
///     // [...]
///     bp
/// }
/// ```
///
/// You can then use the `UrlEncodedBody` extractor as input to your route handlers and constructors.
///
/// # Avoiding allocations
///
/// If you want to minimize memory usage, you can try to avoid unnecessary memory allocations when
/// deserializing string-like fields from the body of the incoming request.
/// Pavex supports this use case—you can borrow from the request body instead of having to
/// allocate a brand new string.
///
/// It is not always possible to avoid allocations, though.
/// In particular, Pavex *must* allocate a new `String` if the Form string you are trying to
/// deserialize contains percent-encoded characters.
/// Using a `&str` in this case would result in a runtime error when attempting the deserialization.
///
/// Given the above, we recommend using `Cow<'_, str>` as field type: it borrows from the request
/// body if possible, and allocates a new `String` only if strictly necessary.
///
/// ```rust
/// use pavex::request::body::UrlEncodedBody;
/// use std::borrow::Cow;
///
/// #[derive(serde::Deserialize)]
/// pub struct Payee<'a> {
///     name: Cow<'a, str>,
/// }
///
/// pub fn get_payee(body: &UrlEncodedBody<Payee<'_>>) -> String {
///    format!("The payee's name is {}", body.0.name)
/// }
/// ```
///
/// # Body size limit
///
/// The `UrlEncodedBody` extractor buffers the entire body in memory before
/// attempting to deserialize it.
///
/// To prevent denial-of-service attacks, Pavex enforces an upper limit on the body size.
/// The limit is enforced by the [`BufferedBody`] extractor,
/// which is injected as one of the inputs of [`FormBody::extract`]. Check out [`BufferedBody`]'s
/// documentation for more details on the size limit (and how to configure it).
///
/// [`BufferedBody`]: super::buffered_body::BufferedBody
pub struct UrlEncodedBody<T>(pub T);

impl<T> UrlEncodedBody<T> {
    pub fn extract<'head, 'body>(
        request_head: &'head RequestHead,
        buffered_body: &'body BufferedBody,
    ) -> Result<Self, ExtractUrlEncodedBodyError>
    where
        'head: 'body,
        T: Deserialize<'body>,
    {
        check_urlencoded_content_type(&request_head.headers)?;

        if request_head.method == Method::GET || request_head.method == Method::HEAD {
            let parse = match request_head.target.query() {
                None => form_urlencoded::parse(&[]),
                Some(s) => form_urlencoded::parse(s.as_bytes()),
            };
            let deserializer = serde_html_form::Deserializer::new(parse);
            let body = serde_path_to_error::deserialize(deserializer).map_err(|e| {
                ExtractUrlEncodedBodyError::QueryDeserializationError(
                    UrlEncodedQueryDeserializationError { source: e },
                )
            })?;
            return Ok(UrlEncodedBody(body));
        } else {
            let bytes = buffered_body.bytes.as_ref();
            let deserializer = serde_html_form::Deserializer::new(form_urlencoded::parse(bytes));
            let body = serde_path_to_error::deserialize(deserializer).map_err(|e| {
                ExtractUrlEncodedBodyError::BodyDeserializationError(
                    UrlEncodedBodyDeserializationError { source: e },
                )
            })?;
            return Ok(UrlEncodedBody(body));
        }
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

#[cfg(test)]
mod tests {
    use crate::request::body::UrlEncodedBody;
    use bytes::Bytes;
    use http::uri::PathAndQuery;
    use http::Uri;

    #[test]
    fn missing_content_type() {
        let headers = http::HeaderMap::new();
        let err = super::check_urlencoded_content_type(&headers).unwrap_err();
        insta::assert_display_snapshot!(err, @"The `Content-Type` header is missing. This endpoint expects requests with a `Content-Type` header set to `application/x-www-form-urlencoded`");
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
        insta::assert_display_snapshot!(err, @"The `Content-Type` header was set to `hello world`. This endpoint expects requests with a `Content-Type` header set to `application/x-www-form-urlencoded`");
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
        insta::assert_display_snapshot!(err, @"The `Content-Type` header was set to `application/json`. This endpoint expects requests with a `Content-Type` header set to `application/x-www-form-urlencoded`");
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
    fn missing_form_field_post() {
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
        insta::assert_display_snapshot!(err, @r###"
        Failed to deserialize the body as a urlencoded form.
        missing field `surname`
        "###);
        insta::assert_debug_snapshot!(err, @r###"
        BodyDeserializationError(
            UrlEncodedBodyDeserializationError {
                source: Error {
                    path: Path {
                        segments: [],
                    },
                    original: Error(
                        "missing field `surname`",
                    ),
                },
            },
        )
        "###);
    }

    #[test]
    /// Let's check the error quality when the request query is missing
    /// a required field.
    fn missing_form_field_get() {
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
            method: http::Method::GET,
            version: http::Version::HTTP_11,
            target: "/?name=John%20Doe&age=43".parse().unwrap(),
        };

        // Act
        let buffered_body = crate::request::body::BufferedBody {
            bytes: Bytes::default(),
        };
        let outcome: Result<UrlEncodedBody<BodySchema>, _> =
            UrlEncodedBody::extract(&request_head, &buffered_body);

        // Assert
        let err = outcome.unwrap_err();
        insta::assert_display_snapshot!(err, @r###"
        Failed to deserialize the query as a urlencoded form.
        missing field `surname`
        "###);
        insta::assert_debug_snapshot!(err, @r###"
        QueryDeserializationError(
            UrlEncodedQueryDeserializationError {
                source: Error {
                    path: Path {
                        segments: [],
                    },
                    original: Error(
                        "missing field `surname`",
                    ),
                },
            },
        )
        "###);
    }
}
