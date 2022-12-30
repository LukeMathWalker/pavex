pub use identifiers::{RawCallable, RawCallableIdentifiers};
pub use variadic_trait::Callable;

mod identifiers;
mod variadic_trait;

#[macro_export]
macro_rules! f {
    ($($p:tt)*) => {{
        pavex_builder::RawCallable {
            import_path: stringify!($($p)*),
            // This is going to raise an error for callables that take generic parameters
            // that have not been specified explicitly using the turbo-fish syntax.
            callable: $($p)*,
        }
    }};
}
