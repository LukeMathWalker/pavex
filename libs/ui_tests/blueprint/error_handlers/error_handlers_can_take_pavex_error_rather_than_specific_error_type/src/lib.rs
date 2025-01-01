use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

#[derive(Debug)]
pub struct CustomError;

pub struct Generic<T> {
    _t: T,
}

impl std::fmt::Display for CustomError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::error::Error for CustomError {}

pub fn fallible_constructor() -> Result<String, CustomError> {
    todo!()
}

pub fn generic_fallible_constructor<T>() -> Result<Generic<T>, CustomError> {
    todo!()
}

pub fn error_handler(_e: &pavex::Error) -> Response {
    todo!()
}

pub fn error_observer(_e: &pavex::Error) {
    todo!()
}

pub fn handler(_s: String, _t: Generic<String>) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(crate::fallible_constructor))
        .error_handler(f!(crate::error_handler));
    bp.request_scoped(f!(crate::generic_fallible_constructor))
        .error_handler(f!(crate::error_handler));

    // We test the behaviour with and without error observers.
    bp.route(GET, "/without_observer", f!(crate::handler));

    bp.nest({
        let mut bp = Blueprint::new();
        bp.error_observer(f!(crate::error_observer));
        bp.route(GET, "/with_observer", f!(crate::handler));
        bp
    });
    bp
}
