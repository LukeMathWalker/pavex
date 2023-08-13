use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;

pub struct GenericType<V>(V);

pub fn generic_wrapping_middleware<T>(generic_input: GenericType<T>) -> u8 {
    todo!()
}

pub fn doubly_generic_wrapping_middleware<T, S>(i1: GenericType<T>, i2: GenericType<S>) -> u16 {
    todo!()
}

pub fn triply_generic_wrapping_middleware<T, S, U>(
    i1: GenericType<T>,
    i2: GenericType<S>,
    i3: GenericType<U>,
) -> u32 {
    todo!()
}

pub fn handler() -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(f!(crate::generic_wrapping_middleware));
    bp.wrap(f!(crate::doubly_generic_wrapping_middleware));
    bp.wrap(f!(crate::triply_generic_wrapping_middleware));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
