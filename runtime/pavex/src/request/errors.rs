use std::fmt::{self, Display};

use crate::{
    Response,
    request::{
        body::errors::{ExtractBufferedBodyError, ExtractJsonBodyError},
        path::errors::ExtractPathParamsError,
        query::errors::ExtractQueryParamsError,
    },
};
use smallvec::SmallVec;

#[derive(Debug)]
pub struct FromRequestErrors {
    items: SmallVec<[ExtractRequestPartError; 4]>,
}

#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum ExtractRequestPartError {
    #[error(transparent)]
    Path(#[from] ExtractPathParamsError),
    #[error(transparent)]
    Query(#[from] ExtractQueryParamsError),
    #[error(transparent)]
    Body(#[from] ExtractBodyError),
}

#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum ExtractBodyError {
    #[error(transparent)]
    BufferedBody(#[from] ExtractBufferedBodyError),
    #[error(transparent)]
    Json(#[from] ExtractJsonBodyError),
}

impl From<ExtractBufferedBodyError> for ExtractRequestPartError {
    fn from(e: ExtractBufferedBodyError) -> Self {
        Self::Body(ExtractBodyError::BufferedBody(e))
    }
}

impl From<ExtractJsonBodyError> for ExtractRequestPartError {
    fn from(e: ExtractJsonBodyError) -> Self {
        Self::Body(ExtractBodyError::Json(e))
    }
}

impl FromRequestErrors {
    pub fn new() -> Self {
        Self {
            items: SmallVec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn push<E>(&mut self, e: E)
    where
        E: Into<ExtractRequestPartError>,
    {
        self.items.push(e.into());
    }

    pub fn iter(&self) -> impl Iterator<Item = &ExtractRequestPartError> + ExactSizeIterator {
        self.items.iter()
    }
}

#[crate::methods]
impl FromRequestErrors {
    #[error_handler(pavex = crate)]
    pub fn to_response(&self) -> Response {
        let mut body =
            "Some parts of the incoming request don't match the expected format:\n".to_string();
        for e in self.items.iter() {
            body.push_str("- ");
            match e {
                ExtractRequestPartError::Path(e) => e.response_body(&mut body),
                ExtractRequestPartError::Query(e) => e.response_body(&mut body),
                ExtractRequestPartError::Body(e) => match e {
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
