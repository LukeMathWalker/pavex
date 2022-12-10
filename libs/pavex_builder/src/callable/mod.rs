pub use identifiers::RawCallableIdentifiers;

mod identifiers;

#[macro_export]
macro_rules! f {
    ($($p:tt)*) => {{
        #[allow(unused_variables)]
        // First we perform a coarse test to try to ensure that $p is a path to a function
        // or a static method.
        // This is going to raise an error for methods that take self as an argument
        // or for functions with generic parameters that have not been specified explicitly
        // using the turbo-fish syntax.
        let callable = $($p)*;
        stringify!($($p)*)
    }};
}
