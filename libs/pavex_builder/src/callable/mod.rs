pub use identifiers::{RawCallable, RawCallableIdentifiers};

mod identifiers;

#[macro_export]
macro_rules! f {
    ($($p:tt)*) => {{
        pavex_builder::RawCallable {
            import_path: stringify!($($p)*),
        }
    }};
}
