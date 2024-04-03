//! Error handling utilities.
pub(crate) mod error_;

/// When things went wrong, but you don't know why.
///
/// `UnexpectedError` is designed for failure scenarios
/// that the application wasn't explicitly prepared to handle.
/// It works, in particular, as the "catch-all" variant in
/// an error enum.
///
/// # Example
///
/// ```rust
/// use pavex::error::UnexpectedError;
/// use pavex::response::Response;
/// # #[derive(Debug)] struct AuthorizationError;
/// # #[derive(Debug)] struct DatabaseError;
/// # impl std::fmt::Display for AuthorizationError {
/// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
/// #         write!(f, "Authorization error")
/// #     }
/// # }
/// # impl std::fmt::Display for DatabaseError {
/// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
/// #         write!(f, "Database error")
/// #     }
/// # }
/// # impl std::error::Error for AuthorizationError {}
/// # impl std::error::Error for DatabaseError {}
///
/// #[derive(Debug, thiserror::Error)]
/// pub enum HandlerError {
///     // One variant for each kind of known issue
///     // that might occur in the request handler.
///     #[error(transparent)]
///     Authorization(#[from] AuthorizationError),
///     #[error(transparent)]
///     Database(#[from] DatabaseError),
///     // [...]
///     // Followed by the catch-all variant.
///     #[error(transparent)]
///     Unexpected(#[from] UnexpectedError),
/// }
///
/// pub async fn request_handler() -> Result<Response, HandlerError> {
///     // [...]
/// # todo!()
/// }
/// ```
///
/// # Error message
///
/// The error message is always the same when using `UnexpectedError`:
/// "An unexpected error occurred".
/// This is intentional, as we don't want to leak any sensitive information
/// or implementation details to the client.
/// The full error details are still available when walking the source error chain and
/// will be captured in your logs if you have a suitable error observer in place.
#[derive(Debug)]
pub struct UnexpectedError {
    inner: Box<dyn std::error::Error + Send + Sync>,
}

impl UnexpectedError {
    /// Create a new [`UnexpectedError`] from a boxable error.
    pub fn new<E>(error: E) -> Self
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Self {
            inner: error.into(),
        }
    }

    /// Convert [`UnexpectedError`] back into the underlying boxed error.
    pub fn into_inner(self) -> Box<dyn std::error::Error + Send + Sync> {
        self.inner
    }

    /// Return a reference to the underlying boxed error.
    pub fn inner_ref(&self) -> &(dyn std::error::Error + Send + Sync) {
        &*self.inner
    }
}

impl std::fmt::Display for UnexpectedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "An unexpected error occurred")
    }
}

impl std::error::Error for UnexpectedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.inner)
    }
}
