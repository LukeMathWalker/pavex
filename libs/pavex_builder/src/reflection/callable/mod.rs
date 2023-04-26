pub use identifiers::{RawCallable, RawCallableIdentifiers};

mod identifiers;

#[macro_export]
macro_rules! f {
    ($($p:tt)*) => {{
        $crate::reflection::RawCallable {
            import_path: stringify!($($p)*),
            registered_at: ::std::env!("CARGO_PKG_NAME", "Failed to load the CARGO_PKG_NAME environment variable. Are you using a custom build system?")
        }
    }};
}
