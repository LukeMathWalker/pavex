use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

pub struct GenericType<V>(V);

pub fn generic_constructor<A>() -> GenericType<A> {
    todo!()
}

pub fn constructor1() -> Result<u32, Error> {
    todo!()
}

pub fn constructor2() -> Result<u16, Error> {
    todo!()
}

pub fn constructor3() -> Result<u8, Error> {
    todo!()
}

pub struct Error;

pub fn generic_error_handler<T>(_error: &Error, _generic_input: GenericType<T>) -> Response {
    todo!()
}

pub fn doubly_generic_error_handler<T, S>(
    _error: &Error,
    _i1: GenericType<T>,
    _i2: GenericType<S>,
) -> Response {
    todo!()
}

pub fn triply_generic_error_handler<T, S, U>(
    _error: &Error,
    _i1: GenericType<T>,
    _i2: GenericType<S>,
    _i3: GenericType<U>,
) -> Response {
    todo!()
}

pub fn handler(_i: u8, _j: u16, _k: u32) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(crate::generic_constructor));
    bp.request_scoped(f!(crate::constructor1))
        .error_handler(f!(crate::generic_error_handler));
    bp.request_scoped(f!(crate::constructor2))
        .error_handler(f!(crate::doubly_generic_error_handler));
    bp.transient(f!(crate::constructor3))
        .error_handler(f!(crate::triply_generic_error_handler));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
