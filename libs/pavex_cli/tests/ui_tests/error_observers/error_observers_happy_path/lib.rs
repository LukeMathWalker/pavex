use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

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

pub fn a() -> A {
    todo!()
}

pub fn b(_a: &A) -> Result<B, ErrorB> {
    todo!()
}

pub fn handler(_b: &B) -> Response {
    todo!()
}

pub fn error_handler(_a: &A, _e: &ErrorB) -> Response {
    todo!()
}

pub fn error_observer(_a: A, _err: &pavex::Error) {
    todo!()
}

pub fn error_observer2(_err: &pavex::Error) {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::a), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::b), Lifecycle::RequestScoped)
        .error_handler(f!(crate::error_handler));
    bp.error_observer(f!(crate::error_observer));
    bp.error_observer(f!(crate::error_observer2));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
