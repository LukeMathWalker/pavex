//! Callable metadata used by Pavex's CLI to analyze your request
//! handlers, constructors and error handlers (e.g. their input parameters, their return type,
//! where they are defined, etc.).
//!
//! This module is not meant to be used directly by users of the framework. It is only meant to be
//! used by Pavex's CLI.

#[derive(Debug, Hash, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
/// An implementation detail of the builder.
/// You must use the [`f!`] macro wherever a `RawCallable` is needed.
///
/// [`f!`]: crate::f
pub struct RawCallable {
    #[doc(hidden)]
    pub import_path: &'static str,
    #[doc(hidden)]
    pub registered_at: &'static str,
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
/// A macro to convert a fully-qualified path into a [`RawCallable`].
///
/// # Guide
///
/// In the ["Cookbook"](https://pavex.dev/docs/guide/dependency_injection/cookbook/)
/// section of Pavex's guide on [dependency injection](https://pavex.dev/docs/guide/dependency_injection/)
/// you can find a collection of reference examples on how to use `f!` macro to register different kinds of
/// callables (functions, methods, trait methods, etc.) with a [`Blueprint`].
///
/// [`Blueprint`]: crate::blueprint::Blueprint
macro_rules! f {
    ($p:expr) => {{
        #[cfg(pavex_ide_hint)]
        const P:() = $p;
        $crate::blueprint::reflection::RawCallable {
            import_path: stringify!($p),
            registered_at: ::std::env!("CARGO_PKG_NAME", "Failed to load the CARGO_PKG_NAME environment variable. Are you using a custom build system?")
        }
    }};
}
