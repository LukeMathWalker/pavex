use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
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

pub fn generic_error_handler<T>(error: &Error, generic_input: GenericType<T>) -> Response {
    todo!()
}

pub fn doubly_generic_error_handler<T, S>(
    error: &Error,
    i1: GenericType<T>,
    i2: GenericType<S>,
) -> Response {
    todo!()
}

pub fn triply_generic_error_handler<T, S, U>(
    error: &Error,
    i1: GenericType<T>,
    i2: GenericType<S>,
    i3: GenericType<U>,
) -> Response {
    todo!()
}

pub fn handler(i: u8, j: u16, k: u32) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::generic_constructor), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::constructor1), Lifecycle::RequestScoped)
        .error_handler(f!(crate::generic_error_handler));
    bp.constructor(f!(crate::constructor2), Lifecycle::RequestScoped)
        .error_handler(f!(crate::doubly_generic_error_handler));
    bp.constructor(f!(crate::constructor3), Lifecycle::Transient)
        .error_handler(f!(crate::triply_generic_error_handler));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
