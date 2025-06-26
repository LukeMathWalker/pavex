use pavex::{blueprint::from, Blueprint};
use pavex::response::Response;

pub struct A;

pub struct B;
pub struct C;

#[derive(Debug)]
pub struct AnError {}

impl std::fmt::Display for AnError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::error::Error for AnError {}

#[pavex::request_scoped(id = "A_")]
pub fn a(_c: &C) -> A {
    todo!()
}

#[pavex::request_scoped(id = "B_")]
pub fn b() -> Result<B, AnError> {
    todo!()
}

#[pavex::request_scoped(id = "C_")]
pub fn c() -> Result<C, AnError> {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler(_b: B) -> Response {
    todo!()
}

#[pavex::error_handler]
pub fn error_handler(_e: &AnError) -> Response {
    todo!()
}

#[pavex::error_observer]
pub fn error_observer(_a: A, _err: &pavex::Error) {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.error_observer(ERROR_OBSERVER);
    bp.routes(from![crate]);
    bp
}
