use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

pub struct A;

pub struct B;

pub struct ErrorB {}

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

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::a), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::b), Lifecycle::RequestScoped)
        .error_handler(f!(crate::error_handler));
    bp.error_observer(f!(crate::error_observer));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
