pub use identifiers::{RawCallable, RawCallableIdentifiers};

mod identifiers;

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
