use pavex_bp_schema::Blueprint as BlueprintSchema;

use super::reflection::AnnotationCoordinates;

/// The input type for [`Blueprint::error_handler`].
///
/// Check out [`Blueprint::error_handler`] for more information on error handling
/// in Pavex.
///
/// # Stability guarantees
///
/// Use the [`error_handler`](macro@crate::error_handler) attribute macro to create instances of `ErrorHandler`.\
/// `ErrorHandler`'s fields are an implementation detail of Pavex's macros and should not be relied upon:
/// newer versions of Pavex may add, remove or modify its fields.
///
/// [`Blueprint::error_handler`]: crate::Blueprint::error_handler
pub struct ErrorHandler {
    #[doc(hidden)]
    pub coordinates: AnnotationCoordinates,
}

/// The type returned by [`Blueprint::error_handler`].
///
/// # Future-proofing
///
/// As of today, [`RegisteredErrorHandler`] doesn't provide any additional functionality.\
/// It is included to allow introducing new configuration for error handlers without having
/// to change the signature of [`Blueprint::error_handler`].
///
/// [`Blueprint::error_handler`]: crate::Blueprint::error_handler
pub struct RegisteredErrorHandler<'a> {
    #[allow(dead_code)]
    pub(crate) blueprint: &'a mut BlueprintSchema,
}
