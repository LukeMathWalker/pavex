//! Metadata used by Pavex's CLI to analyze your request
//! handlers, constructors, error handlers, error observers (e.g. their input parameters, their return type,
//! where they are defined, etc.), etc.
//!
//! This module is not meant to be used directly by users of the framework. It is only meant to be
//! used by Pavex's CLI.

#[derive(Debug, Hash, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
/// An implementation detail of the builder.
/// You must use the [`f!`] macro wherever an instance of `RawIdentifiers` is needed.
///
/// [`f!`]: crate::f
pub struct RawIdentifiers {
    #[doc(hidden)]
    pub import_path: &'static str,
    #[doc(hidden)]
    pub crate_name: &'static str,
    #[doc(hidden)]
    pub module_path: &'static str,
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
/// A macro to convert an [unambiguous path](https://pavex.dev/docs/guide/dependency_injection/cookbook/#unambiguous-paths)
/// into [`RawIdentifiers`].
///
/// # Guide
///
/// In the ["Cookbook"](https://pavex.dev/docs/guide/dependency_injection/cookbook/)
/// section of Pavex's guide on [dependency injection](https://pavex.dev/docs/guide/dependency_injection/)
/// you can find a collection of reference examples on how to use `f!` macro to register different kinds of
/// callables (functions, methods, trait methods, etc.) and types with a [`Blueprint`].
///
/// [`Blueprint`]: crate::blueprint::Blueprint
macro_rules! f {
    // This branch is used by `Blueprint::state_input`, where you need to specifically
    // pass a type path to the macro.
    // The `ty` designator is more restrictive than the `expr` designator, so it's
    // the first one we try to match.
    ($t:ty) => {{
        #[cfg(pavex_ide_hint)]
        const P:$t = ();
        $crate::blueprint::reflection::RawIdentifiers {
            import_path: stringify!($t),
            crate_name: ::std::env!("CARGO_PKG_NAME", "Failed to load the CARGO_PKG_NAME environment variable. Are you using a custom build system?"),
            module_path: module_path!()
        }
    }};
    // This branch is used in most cases, where you are instead specifying the path
    // to a callable.
    ($p:expr) => {{
        #[cfg(pavex_ide_hint)]
        const P:() = $p;
        $crate::blueprint::reflection::RawIdentifiers {
            import_path: stringify!($p),
            crate_name: ::std::env!("CARGO_PKG_NAME", "Failed to load the CARGO_PKG_NAME environment variable. Are you using a custom build system?"),
            module_path: module_path!()
        }
    }};
}
