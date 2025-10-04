use pavex_bp_schema::Blueprint as BlueprintSchema;

use super::reflection::AnnotationCoordinates;

/// The input type for [`Blueprint::error_observer`].
///
/// Check out [`Blueprint::error_observer`] for more information on error handling
/// in Pavex.
///
/// # Stability guarantees
///
/// Use the [`error_observer`](macro@crate::error_observer) attribute macro to create instances of `ErrorObserver`.\
/// `ErrorObserver`'s fields are an implementation detail of Pavex's macros and should not be relied upon:
/// newer versions of Pavex may add, remove or modify its fields.
///
/// [`Blueprint::error_observer`]: crate::Blueprint::error_observer
pub struct ErrorObserver {
    #[doc(hidden)]
    pub coordinates: AnnotationCoordinates,
}

/// The type returned by [`Blueprint::error_observer`].
///
/// # Future-proofing
///
/// As of today, [`RegisteredErrorObserver`] doesn't provide any additional functionality.\
/// It is included to allow introducing new configuration for error observers without having
/// to change the signature of [`Blueprint::error_observer`].
///
/// [`Blueprint::error_observer`]: crate::Blueprint::error_observer
pub struct RegisteredErrorObserver<'a> {
    #[allow(dead_code)]
    pub(crate) blueprint: &'a mut BlueprintSchema,
}
