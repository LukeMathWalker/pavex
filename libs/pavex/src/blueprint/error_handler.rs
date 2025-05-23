//! Register error handlers.
//!
//! # Guide
//!
//! Check out the ["Error handlers"](https://pavex.dev/docs/guide/errors/error_handlers)
//! section of Pavex's guide for a thorough introduction to error handlers
//! in Pavex applications.
use pavex_bp_schema::Blueprint as BlueprintSchema;

/// The type returned by [`Blueprint::error_handler`].
///
/// It allows you to further configure the behaviour of the registered error handler.
///
/// [`Blueprint::error_handler`]: crate::blueprint::Blueprint::error_handler
pub struct RegisteredErrorHandler<'a> {
    #[allow(dead_code)]
    pub(crate) blueprint: &'a mut BlueprintSchema,
}
