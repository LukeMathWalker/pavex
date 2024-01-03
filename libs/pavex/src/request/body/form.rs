use http::HeaderMap;
use serde::Deserialize;

use crate::request::RequestHead;

use super::{
    buffered_body::BufferedBody,
    errors::{
        ExtractFormBodyError, FormContentTypeMismatch, FormDeserializationError,
        MissingFormContentType,
    },
};

#[doc(alias = "Form")]
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
/// use pavex::request::body::FormBody;
///
/// // You must derive `serde::Deserialize` for the type you want to extract,
/// // in this case `HomeListing`.
/// #[derive(serde::Deserialize)]
/// pub struct HomeListing {
///     address: String,
///     price: u64,
/// }
///
/// // The `Form` extractor deserializes the request body into
/// // the type you specified—`HomeListing` in this case.
/// pub fn get_home(body: &FormBody<HomeListing>) -> String {
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
/// `FormBody` in your `Blueprint`:
///
/// ```rust
/// use pavex::f;
/// use pavex::blueprint::{Blueprint, constructor::Lifecycle};
///
/// fn blueprint() -> Blueprint {
///    let mut bp = Blueprint::new();
///    // Register the default constructor and error handler for `FormBody`.
///    bp.constructor(
///         f!(pavex::request::body::FormBody::extract),
///         Lifecycle::RequestScoped,
///     ).error_handler(
///         f!(pavex::request::body::errors::ExtractFormBodyError::into_response)
///     );
///     // [...]
///     bp
/// }
/// ```
///
/// You can then use the `FormBody` extractor as input to your route handlers and constructors.
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
/// use pavex::request::body::FormBody;
/// use std::borrow::Cow;
///
/// #[derive(serde::Deserialize)]
/// pub struct Payee<'a> {
///     name: Cow<'a, str>,
/// }
///
/// pub fn get_payee(body: &FormBody<Payee<'_>>) -> String {
///    format!("The payee's name is {}", body.0.name)
/// }
/// ```
///
/// # Body size limit
///
/// The `FormBody` extractor buffers the entire body in memory before
/// attempting to deserialize it.
///
/// To prevent denial-of-service attacks, Pavex enforces an upper limit on the body size.
/// The limit is enforced by the [`BufferedBody`] extractor,
/// which is injected as one of the inputs of [`FormBody::extract`]. Check out [`BufferedBody`]'s
/// documentation for more details on the size limit (and how to configure it).
///
/// [`BufferedBody`]: super::buffered_body::BufferedBody
pub struct FormBody<T>(pub T);

impl<T> FormBody<T> {
    /// The default constructor for [`FormBody`].
    ///
    /// The extraction can fail for a number of reasons:
    ///
    /// - the `Content-Type` is missing
    /// - the `Content-Type` header is not set to `application/x-www-form-urlencoded`
    /// - the request body is not a valid form
    ///
    /// In all of the above cases, an [`ExtractFormBodyError`] is returned.
    // # Implementation notes
    //
    // We are using two separate lifetimes here to make it clear to the compiler
    // that `FormBody` doesn't borrow from `RequestHead`.
    pub fn extract<'head, 'body>(
        request_head: &'head RequestHead,
        buffered_body: &'body BufferedBody,
    ) -> Result<Self, ExtractFormBodyError>
    where
        T: Deserialize<'body>,
    {
        check_form_content_type(&request_head.headers)?;
        let deserializer = serde_html_form::Deserializer::new(form_urlencoded::parse(
            buffered_body.bytes.as_ref(),
        ));
        let body = serde_path_to_error::deserialize(deserializer)
            .map_err(|e| FormDeserializationError { source: e })?;
        Ok(FormBody(body))
    }
}

/// Check that the `Content-Type` header is set to `application/x-www-form-urlencoded`.
///
/// Return an error otherwise.
fn check_form_content_type(headers: &HeaderMap) -> Result<(), ExtractFormBodyError> {
    let Some(content_type) = headers.get(http::header::CONTENT_TYPE) else {
        return Err(MissingFormContentType.into());
    };
    let Ok(content_type) = content_type.to_str() else {
        return Err(MissingFormContentType.into());
    };

    let Ok(mime) = content_type.parse::<mime::Mime>() else {
        return Err(FormContentTypeMismatch {
            actual: content_type.to_string(),
        }
        .into());
    };

    let is_form_content_type =
        mime.type_() == mime::APPLICATION && mime.subtype() == mime::WWW_FORM_URLENCODED;
    if !is_form_content_type {
        return Err(FormContentTypeMismatch {
            actual: content_type.to_string(),
        }
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::request::body::FormBody;

    #[test]
    fn missing_content_type() {
        let headers = http::HeaderMap::new();
        let err = super::check_form_content_type(&headers).unwrap_err();
        insta::assert_display_snapshot!(err, @"The `Content-Type` header is missing. This endpoint expects requests with a `Content-Type` header set to `application/x-www-form-urlencoded`");
        insta::assert_debug_snapshot!(err, @r###"
        MissingContentType(
            MissingFormContentType,
        )
        "###);
    }

    #[test]
    fn content_type_is_not_valid_mime() {
        let mut headers = http::HeaderMap::new();
        headers.insert(http::header::CONTENT_TYPE, "hello world".parse().unwrap());

        let err = super::check_form_content_type(&headers).unwrap_err();
        insta::assert_display_snapshot!(err, @"The `Content-Type` header was set to `hello world`. This endpoint expects requests with a `Content-Type` header set to `application/x-www-form-urlencoded`");
        insta::assert_debug_snapshot!(err, @r###"
        ContentTypeMismatch(
            FormContentTypeMismatch {
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

        let err = super::check_form_content_type(&headers).unwrap_err();
        insta::assert_display_snapshot!(err, @"The `Content-Type` header was set to `application/json`. This endpoint expects requests with a `Content-Type` header set to `application/x-www-form-urlencoded`");
        insta::assert_debug_snapshot!(err, @r###"
        ContentTypeMismatch(
            FormContentTypeMismatch {
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

        let outcome = super::check_form_content_type(&headers);
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

        let outcome = super::check_form_content_type(&headers);
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
            method: http::Method::GET,
            uri: "/".parse().unwrap(),
            version: http::Version::HTTP_11,
        };
        let body = "name=John%20Doe&age=43".to_string();

        // Act
        let buffered_body = crate::request::body::BufferedBody { bytes: body.into() };
        let outcome: Result<FormBody<BodySchema>, _> =
            FormBody::extract(&request_head, &buffered_body);

        // Assert
        let err = outcome.unwrap_err();
        insta::assert_display_snapshot!(err, @r###"
        Failed to deserialize the body as a form.
        missing field `surname`
        "###);
        insta::assert_debug_snapshot!(err, @r###"
        DeserializationError(
            FormDeserializationError {
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
