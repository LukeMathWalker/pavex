//! Register error observers to intercept and report errors that occur during request handling.
//!
//! # Guide
//!
//! Check out the ["Error observers"](https://pavex.dev/docs/guide/errors/error_observers)
//! section of Pavex's guide for a thorough introduction to error observers
//! in Pavex applications.
use pavex_bp_schema::Blueprint as BlueprintSchema;

/// The type returned by [`Blueprint::error_observer`].
///
/// It allows you to further configure the behaviour of the registered error observer.
pub struct RegisteredErrorObserver<'a> {
    #[allow(dead_code)]
    pub(crate) blueprint: &'a mut BlueprintSchema,
}
