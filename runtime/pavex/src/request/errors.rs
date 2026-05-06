use std::fmt::{self, Display};

use crate::{
    Response,
    request::{
        body::errors::ExtractBodyError, path::errors::ExtractPathParamsError,
        query::errors::ExtractQueryParamsError,
    },
};
use smallvec::SmallVec;

/// A collection of [`FromRequestError`] instances that occurred while trying to extract information
/// from an incoming request.
///
/// [`FromRequestErrors`] is designed to improve the debugging experience of your API users.
/// The API caller is informed about multiple issues **at once**,
/// thus reducing the number of iterations required to fix the underlying issues.
#[derive(Debug, Default)]
pub struct FromRequestErrors {
    items: SmallVec<[FromRequestError; 4]>,
}

/// Something went wrong while trying to extract information from an incoming request.
///
/// [`FromRequestError`] wraps the different types of extraction errors that can
/// occur when processing a request when using Pavex's built-in extractors:
/// - [`ExtractPathParamsError`] for path parameters
/// - [`ExtractQueryParamsError`] for query parameters
/// - [`ExtractBodyError`] for request bodies
///
/// This type is rarely used directly. Multiple [`FromRequestError`] instances are
/// collected into a [`FromRequestErrors`] to provide a unified error report to
/// the API caller.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum FromRequestError {
    #[error(transparent)]
    /// See [`ExtractPathParamsError`] for details.
    Path(#[from] ExtractPathParamsError),
    #[error(transparent)]
    /// See [`ExtractQueryParamsError`] for details.
    Query(#[from] ExtractQueryParamsError),
    #[error(transparent)]
    /// See [`ExtractBodyError`] for details.
    Body(#[from] ExtractBodyError),
}

impl FromRequestErrors {
    /// Create a new, empty [`FromRequestErrors`] collection.
    ///
    /// This is typically used at the start of request extraction to initialize an error
    /// accumulator that will collect any extraction failures that occur.
    ///
    /// Errors can be added to the collection using [`push`].
    ///
    /// [`push`]: FromRequestErrors::push
    pub fn new() -> Self {
        Self {
            items: SmallVec::new(),
        }
    }

    /// Returns `true` if there are no errors in the collection.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the number of errors that have been collected so far.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Add a new error to the collection.
    ///
    /// The error can be any type that can be converted into a [`FromRequestError`]—this includes
    /// [`ExtractPathParamsError`], [`ExtractQueryParamsError`], [`ExtractJsonBodyError`], and
    /// [`ExtractBufferedBodyError`].
    pub fn push<E>(&mut self, e: E)
    where
        E: Into<FromRequestError>,
    {
        self.items.push(e.into());
    }

    /// Returns an iterator over the collected errors.
    ///
    /// The iterator yields references to each [`FromRequestError`] in the order they were added.
    pub fn iter(&self) -> impl Iterator<Item = &FromRequestError> + ExactSizeIterator {
        self.items.iter()
    }
}

#[crate::methods]
impl FromRequestErrors {
    /// Convert [`FromRequestErrors`] into an HTTP response.
    ///
    /// This method generates a `400 Bad Request` response with a plain-text body.
    /// The body lists all the extraction errors that occurred. Each error is presented on a separate line, making it
    /// easy for API users to understand what went wrong with their request.
    ///
    /// Pavex uses this method as the default error handler for [`FromRequestErrors`].
    #[error_handler(pavex = crate)]
    pub fn to_response(&self) -> Response {
        let mut body =
            "Some parts of the incoming request don't match the expected format:\n".to_string();
        for e in self.items.iter() {
            body.push_str("- ");
            match e {
                FromRequestError::Path(e) => e.response_body(&mut body),
                FromRequestError::Query(e) => e.response_body(&mut body),
                FromRequestError::Body(e) => match e {
                    ExtractBodyError::BufferedBody(e) => e.response_body(&mut body),
                    ExtractBodyError::Json(e) => e.response_body(&mut body),
                },
            }
            .expect("Failed to write into a string buffer");
        }
        Response::bad_request().set_typed_body(body)
    }
}

impl Display for FromRequestErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.items.is_empty() {
            return write!(f, "No request extraction errors");
        }
        writeln!(f, "Something went wrong during request extraction:")?;
        for e in self.items.iter() {
            writeln!(f, "- {e}")?;
        }
        Ok(())
    }
}
