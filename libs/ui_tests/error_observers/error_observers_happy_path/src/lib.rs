use pavex::{blueprint::from, Blueprint};
use pavex::Response;

pub struct A;

pub struct B;

#[derive(Debug)]
pub struct ErrorB {}

impl std::fmt::Display for ErrorB {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::error::Error for ErrorB {}

#[pavex::request_scoped(id = "A_")]
pub fn a() -> A {
    todo!()
}

#[pavex::request_scoped(id = "B_")]
pub fn b(_a: &A) -> Result<B, ErrorB> {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler(_b: &B) -> Response {
    todo!()
}

#[pavex::error_handler]
pub fn error_handler(_a: &A, #[px(error_ref)] _e: &ErrorB) -> Response {
    todo!()
}

#[pavex::error_observer]
pub fn error_observer(_a: &A, _err: &pavex::Error) {
    todo!()
}

#[pavex::error_observer]
pub fn error_observer2(_err: &pavex::Error) {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.error_observer(ERROR_OBSERVER);
    bp.error_observer(ERROR_OBSERVER_2);
    bp.routes(from![crate]);
    bp
}
