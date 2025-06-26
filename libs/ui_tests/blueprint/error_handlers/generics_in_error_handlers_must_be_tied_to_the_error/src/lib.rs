use pavex::response::Response;
use pavex::{blueprint::from, Blueprint};

pub struct GenericType<V>(V);

#[pavex::request_scoped]
pub fn generic_constructor<A>() -> GenericType<A> {
    todo!()
}

#[pavex::request_scoped]
pub fn constructor1() -> Result<u32, Error> {
    todo!()
}

#[pavex::request_scoped]
pub fn constructor2() -> Result<u16, Error> {
    todo!()
}

#[pavex::request_scoped]
pub fn constructor3() -> Result<u8, Error> {
    todo!()
}

pub struct Error;

#[pavex::error_handler(default = false)]
pub fn generic_error_handler<T>(
    #[px(error_ref)] _error: &Error,
    _generic_input: GenericType<T>,
) -> Response {
    todo!()
}

#[pavex::error_handler(default = false)]
pub fn doubly_generic_error_handler<T, S>(
    #[px(error_ref)] _error: &Error,
    _i1: GenericType<T>,
    _i2: GenericType<S>,
) -> Response {
    todo!()
}

#[pavex::error_handler(default = false)]
pub fn triply_generic_error_handler<T, S, U>(
    #[px(error_ref)] _error: &Error,
    _i1: GenericType<T>,
    _i2: GenericType<S>,
    _i3: GenericType<U>,
) -> Response {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler(_i: u8, _j: u16, _k: u32) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(GENERIC_CONSTRUCTOR);
    bp.constructor(CONSTRUCTOR_1)
        .error_handler(GENERIC_ERROR_HANDLER);
    bp.constructor(CONSTRUCTOR_2)
        .error_handler(DOUBLY_GENERIC_ERROR_HANDLER);
    bp.constructor(CONSTRUCTOR_3)
        .error_handler(TRIPLY_GENERIC_ERROR_HANDLER);
    bp.routes(from![crate]);
    bp
}
