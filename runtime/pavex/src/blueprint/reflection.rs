//! Metadata used by Pavex's CLI to analyze your request
//! handlers, constructors, error handlers, error observers (e.g. their input parameters, their return type,
//! where they are defined, etc.), etc.
//!
//! This module is not meant to be used directly by users of the framework. It is only meant to be
//! used by Pavex's CLI.
use std::borrow::Cow;

/// The modules you want to import components from.
pub enum Sources {
    /// Use all valid sources: all components defined in the current crate and its direct dependencies.
    All,
    /// Use only the specified modules as sources.
    ///
    /// Each module can be either from the current crate or from one of its direct dependencies.
    Some(Vec<Cow<'static, str>>),
}

/// Metadata about an annotated component.
///
/// It is used by Pavex to retrieve the component's location and match the information in the annotation
/// with those provided when the component was registered with the [`Blueprint`](crate::Blueprint).
///
/// # Stability
///
/// Newer versions of Pavex may introduce, remove or modify the fields of this struct—it is considered
/// an implementation detail of Pavex's macros and should not be used directly.
#[derive(Debug)]
pub struct AnnotationCoordinates {
    #[doc(hidden)]
    /// The identifier of the component.
    ///
    /// It must be unique within the crate where the component was defined.
    pub id: &'static str,
    #[doc(hidden)]
    /// Metadata to identify the location where the component was defined.
    pub created_at: CreatedAt,
    #[doc(hidden)]
    /// The name of the macro attribute used to create the component.
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
    /// The name of the Cargo package where the value was created.
    #[doc(hidden)]
    pub package_name: &'static str,
    /// The version of the Cargo package where the value was created.
    #[doc(hidden)]
    pub package_version: &'static str,
}

#[macro_export]
#[doc(hidden)]
/// A convenience macro to initialize a [`CreatedAt`] instance.
macro_rules! created_at {
    () => {
        $crate::blueprint::reflection::CreatedAt {
            package_name: ::std::env!("CARGO_PKG_NAME", "Failed to load the CARGO_PKG_NAME environment variable. Are you using a custom build system?"),
            package_version: ::std::env!("CARGO_PKG_VERSION", "Failed to load the CARGO_PKG_VERSION environment variable. Are you using a custom build system?"),
        }
    };
}
