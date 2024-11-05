use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;

pub struct GenericType<V>(V);

pub fn generic_constructor<T>(_generic_input: GenericType<T>) -> u8 {
    todo!()
}

pub fn doubly_generic_constructor<T, S>(_i1: GenericType<T>, _i2: GenericType<S>) -> u16 {
    todo!()
}

pub fn triply_generic_constructor<T, S, U>(
    _i1: GenericType<T>,
    _i2: GenericType<S>,
    _i3: GenericType<U>,
) -> u32 {
    todo!()
}

pub fn handler(_i: u8, _j: u16, _k: u32) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(crate::generic_constructor));
    bp.request_scoped(f!(crate::doubly_generic_constructor));
    bp.request_scoped(f!(crate::triply_generic_constructor));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
