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
/// `WithLocation` is populated by Pavex's macros—[`f!`], [`t!`] and [`from!`].
/// Newer versions of Pavex may introduce, remove or modify the fields of this struct—it is considered
/// an implementation detail of Pavex's macros and should not be used directly.
///
/// [`from!`]: super::from
/// [`f!`]: crate::f
/// [`t!`]: crate::t
pub struct WithLocation<T> {
    /// The decorated value.
    pub value: T,
    /// The location where the value was created.
    pub created_at: CreatedAt,
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
/// `CreatedAt` fields are always populated by Pavex's macros—[`f!`], [`t!`] and [`from!`].
/// Newer versions of Pavex may introduce, remove or modify the fields of this struct—it is considered
/// an implementation detail of Pavex's macros and should not be used directly.
///
/// [`from!`]: super::from
/// [`f!`]: crate::f
/// [`t!`]: crate::t
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
macro_rules! with_location {
    ($value:expr) => {
        $crate::blueprint::reflection::WithLocation {
            value: $value,
            created_at: $crate::blueprint::reflection::CreatedAt {
                package_name: ::std::env!("CARGO_PKG_NAME", "Failed to load the CARGO_PKG_NAME environment variable. Are you using a custom build system?"),
                package_version: ::std::env!("CARGO_PKG_VERSION", "Failed to load the CARGO_PKG_VERSION environment variable. Are you using a custom build system?"),
                module_path: module_path!(),
            },
        }
    };
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
/// An implementation detail of the builder.
/// You must use the [`f!`] macro wherever an instance of `RawIdentifiers` is needed.
///
/// [`f!`]: crate::f
pub struct RawIdentifiers {
    #[doc(hidden)]
    pub import_path: &'static str,
    #[doc(hidden)]
    pub macro_name: &'static str,
}

// The `pavex_ide_hint`-let binding is a hack to "nudge"
// rust-analyzer into parsing the macro input
// as an expression path, therefore enabling auto-completion and
// go-to-definition for the path that's passed as input.
//
// Rust-analyzer doesn't do this by default because it can't infer
// from the macro-generated code what the macro input "looks like".
// `stringify` accepts anything as input, so that's not enough.
//
// The perma-disabled `let` binding lets us workaround the issue
// that an _actual_ `let` binding would cause: it would fail to
// compile if the callable is generic, because the compiler would
// demand to know the type of each generic parameter without a default.
#[macro_export]
/// Convert an [unambiguous callable path](https://pavex.dev/docs/guide/dependency_injection/cookbook/#unambiguous-paths)
/// into [`RawIdentifiers`].
///
/// `f!` is a short-hand for "function-like". It's the macro used to specify a function or a method
/// to be used as a constructor, request handler, etc.
/// Use [`t!`], instead, to specify a type when invoking [`Blueprint::prebuilt`].
///
/// # Guide
///
/// In the ["Cookbook"](https://pavex.dev/docs/guide/dependency_injection/cookbook/)
/// section of Pavex's guide on [dependency injection](https://pavex.dev/docs/guide/dependency_injection/)
/// you can find a collection of reference examples on how to use `f!` macro to register different kinds of
/// callables (functions, methods, trait methods, etc.) with a [`Blueprint`].
///
/// [`Blueprint`]: crate::blueprint::Blueprint
/// [`Blueprint::prebuilt`]: crate::blueprint::Blueprint::prebuilt
/// [`t!`]: crate::t
macro_rules! f {
    ($p:expr) => {{
        #[cfg(pavex_ide_hint)]
        let x = $p();

        $crate::with_location!($crate::blueprint::reflection::RawIdentifiers {
            import_path: stringify!($p),
            macro_name: "f",
        })
    }};
}

#[macro_export]
/// Convert an [unambiguous type path](https://pavex.dev/docs/guide/dependency_injection/cookbook/#unambiguous-paths)
/// into [`RawIdentifiers`].
///
/// `t!` is a short-hand for "type". It's the macro used by [`Blueprint::prebuilt`].
/// You should use [`f!`] if you're invoking other methods on [`Blueprint`].
///
/// # Guide
///
/// In the ["Cookbook"](https://pavex.dev/docs/guide/dependency_injection/cookbook/)
/// section of Pavex's guide on [dependency injection](https://pavex.dev/docs/guide/dependency_injection/)
/// you can find a collection of reference examples on how to use `t!` macro to register different kinds of
/// types (generic, with lifetimes, etc.) as state inputs with a [`Blueprint`].
///
/// [`Blueprint`]: crate::blueprint::Blueprint
/// [`Blueprint::prebuilt`]: crate::blueprint::Blueprint::prebuilt
/// [`f!`]: crate::f
macro_rules! t {
    // This branch is used by `Blueprint::prebuilt`, where you need to specifically
    // pass a type path to the macro.
    // The `ty` designator is more restrictive than the `expr` designator, so it's
    // the first one we try to match.
    ($t:ty) => {{
        #[cfg(pavex_ide_hint)]
        const P: $t;
        $crate::with_location!($crate::blueprint::reflection::RawIdentifiers {
            import_path: stringify!($t),
            macro_name: "t",
        })
    }};
}
