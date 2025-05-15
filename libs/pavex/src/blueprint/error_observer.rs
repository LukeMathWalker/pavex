//! Register error observers to intercept and report errors that occur during request handling.
//!
//! # Guide
//!
//! Check out the ["Error observers"](https://pavex.dev/docs/guide/errors/error_observers)
//! section of Pavex's guide for a thorough introduction to error observers
//! in Pavex applications.
use crate::blueprint::Blueprint;
use crate::blueprint::conversions::raw_identifiers2callable;
use crate::blueprint::reflection::RawIdentifiers;
use pavex_bp_schema::{Blueprint as BlueprintSchema, Callable};

use super::reflection::WithLocation;

/// The type returned by [`Blueprint::error_observer`].
///
/// It allows you to further configure the behaviour of the registered error observer.
pub struct RegisteredErrorObserver<'a> {
    #[allow(dead_code)]
    pub(crate) blueprint: &'a mut BlueprintSchema,
}

/// An error observer that has been configured but has not yet been registered with a [`Blueprint`].
///
/// # Guide
///
/// Check out [`Blueprint::error_observer`] for an introduction to error observers in Pavex.
#[derive(Clone, Debug)]
pub struct ErrorObserver {
    pub(in crate::blueprint) callable: Callable,
}

impl ErrorObserver {
    /// Create a new (unregistered) error observer.
    ///
    /// Check out the documentation of [`Blueprint::error_observer`] for more details
    /// on observers.
    #[track_caller]
    pub fn new(callable: WithLocation<RawIdentifiers>) -> Self {
        Self {
            callable: raw_identifiers2callable(callable),
        }
    }

    /// Register this error observer with a [`Blueprint`].
    ///
    /// Check out the documentation of [`Blueprint::error_observer`] for more details.
    pub fn register(self, bp: &mut Blueprint) -> RegisteredErrorObserver {
        bp.register_error_observer(self)
    }
}
