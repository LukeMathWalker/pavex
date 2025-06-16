//! Metadata used by Pavex's CLI to analyze your request
//! handlers, constructors, error handlers, error observers (e.g. their input parameters, their return type,
//! where they are defined, etc.), etc.
//!
//! This module is not meant to be used directly by users of the framework. It is only meant to be
//! used by Pavex's CLI.
use std::borrow::Cow;

/// Attach [location metadata](CreatedAt) to a value.
///
/// # Stability
///
/// `WithLocation` is populated by Pavex's [`from!`] macro.
/// Newer versions of Pavex may introduce, remove or modify the fields of this struct—it is considered
/// an implementation detail of Pavex's macros and should not be used directly.
///
/// [`from!`]: super::from
#[derive(Debug)]
pub struct WithLocation<T> {
    /// The decorated value.
    pub value: T,
    /// The location where the value was created.
    pub created_at: CreatedAt,
}

pub struct AnnotationCoordinates {
    pub id: &'static str,
    pub created_at: CreatedAt,
    pub macro_name: &'static str,
}

/// Metadata about the module where a Pavex macro was invoked to create a component.
///
/// It is used by Pavex to:
///
/// - unambiguously identify in which crate of your dependency tree a component was created
/// - convert relative import paths to absolute paths starting from the root of the relevant crate
///
/// # Stability
///
/// `CreatedAt` fields are always populated by Pavex's macros.
/// Newer versions of Pavex may introduce, remove or modify the fields of this struct—it is considered
/// an implementation detail of Pavex's macros and should not be used directly.
#[derive(Debug)]
pub struct CreatedAt {
    /// The name of the Cargo package where the value within [`WithLocation`] was created.
    pub package_name: &'static str,
    /// The version of the Cargo package where the value within [`WithLocation`] was created.
    pub package_version: &'static str,
    /// The value returned by `std`'s `module_path!` macro.
    pub module_path: &'static str,
}

/// The type returned by invocations of the [`from!`] macro.
///
/// # Stability
///
/// `Sources` is always populated by the [`from!`] macro.
/// Newer versions of Pavex may introduce, remove or modify the fields of this type—it is considered
/// an implementation detail of [`from!`] macros and should not be used directly.
///
/// Invoke the [`from!`] macro wherever an instance of `Sources` is needed.
///
/// [`from!`]: super::from
pub enum Sources {
    /// Use all valid sources: modules from the current crate and all its direct dependencies.
    All,
    /// Use only the specified modules as sources.
    ///
    /// Each module can be either from the current crate or from one of its direct dependencies.
    Some(Vec<Cow<'static, str>>),
}

#[macro_export]
#[doc(hidden)]
/// A convenience macro to create a [`WithLocation`] instance.
macro_rules! created_at {
    () => {
        $crate::blueprint::reflection::CreatedAt {
            package_name: ::std::env!("CARGO_PKG_NAME", "Failed to load the CARGO_PKG_NAME environment variable. Are you using a custom build system?"),
            package_version: ::std::env!("CARGO_PKG_VERSION", "Failed to load the CARGO_PKG_VERSION environment variable. Are you using a custom build system?"),
            module_path: module_path!(),
        }
    };
}

#[macro_export]
#[doc(hidden)]
/// A convenience macro to create a [`WithLocation`] instance.
macro_rules! with_location {
    ($value:expr) => {
        $crate::blueprint::reflection::WithLocation {
            value: $value,
            created_at: $crate::created_at!(),
        }
    };
}
