use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

pub struct A;

pub struct B;

#[derive(Debug)]
pub struct AnError {}

impl std::fmt::Display for AnError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::error::Error for AnError {}

pub fn a() -> Result<A, AnError> {
    todo!()
}

pub fn b() -> Result<B, AnError> {
    todo!()
}

pub fn handler(_b: B) -> Response {
    todo!()
}

pub fn error_handler(_e: &AnError) -> Response {
    todo!()
}

pub fn error_observer(_a: A, _err: &pavex::Error) {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::a), Lifecycle::RequestScoped)
        .error_handler(f!(crate::error_handler));
    bp.constructor(f!(crate::b), Lifecycle::RequestScoped)
        .error_handler(f!(crate::error_handler));
    bp.error_observer(f!(crate::error_observer));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
