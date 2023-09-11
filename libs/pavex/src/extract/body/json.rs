use http::HeaderMap;
use serde::Deserialize;

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
/// use pavex::extract::body::JsonBody;
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
///
/// # Installation
///
/// First of all, you need the register the default constructor and error handler for
/// `JsonBody` in your `Blueprint`:
///
/// ```rust
/// use pavex::f;
/// use pavex::blueprint::{Blueprint, constructor::Lifecycle};
///
/// fn blueprint() -> Blueprint {
///    let mut bp = Blueprint::new();
///    // Register the default constructor and error handler for `JsonBody`.
///    bp.constructor(
///         f!(pavex::extract::body::JsonBody::extract),
///         Lifecycle::RequestScoped,
///     ).error_handler(
///         f!(pavex::extract::body::errors::ExtractJsonBodyError::into_response)
///     );
///     // [...]
///     bp
/// }
/// ```
///
/// You can then use the `JsonBody` extractor as input to your route handlers and constructors.
///
/// # Avoiding allocations
///
/// If you want to minimize memory usage, you can try to avoid unnecessary memory allocations when
/// deserializing string-like fields from the body of the incoming request.    
/// Pavex supports this use case—you can borrow from the request body instead of having to
/// allocate a brand new string.
///
/// It is not always possible to avoid allocations, though.  
/// In particular, Pavex *must* allocate a new `String` if the JSON string you are trying to
/// deserialize contains escape sequences, such as `\n` or `\"`.
/// Using a `&str` in this case would result in a runtime error when attempting the deserialization.
///
/// Given the above, we recommend using `Cow<'_, str>` as field type: it borrows from the request
/// body if possible, and allocates a new `String` only if strictly necessary.
///
/// ```rust
/// use pavex::extract::body::JsonBody;
/// use std::borrow::Cow;
///
/// #[derive(serde::Deserialize)]
/// pub struct Payee<'a> {
///     name: Cow<'a, str>,
/// }
///
/// pub fn get_payee(body: &JsonBody<Payee<'_>>) -> String {
///    format!("The payee's name is {}", body.0.name)
/// }
/// ```
///
/// # Body size limit
///
/// The `JsonBody` extractor buffers the entire body in memory before
/// attempting to deserialize it.  
///
/// To prevent denial-of-service attacks, Pavex enforces an upper limit on the body size.    
/// The limit is enforced by the [`BufferedBody`] extractor,
/// which is injected as one of the inputs of [`JsonBody::extract`]. Check out [`BufferedBody`]'s
/// documentation for more details on the size limit (and how to configure it).
///
/// [`BufferedBody`]: super::buffered_body::BufferedBody
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
    use crate::extract::body::JsonBody;

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
            uri: "/".parse().unwrap(),
            version: http::Version::HTTP_11,
        };
        let body = serde_json::json!({
            "name": "John Doe",
            "age": 43,
        });

        // Act
        let buffered_body = crate::extract::body::BufferedBody {
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
