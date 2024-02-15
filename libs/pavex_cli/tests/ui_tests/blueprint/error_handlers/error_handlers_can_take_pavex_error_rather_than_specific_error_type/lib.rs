use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

#[derive(Debug)]
pub struct CustomError;

impl std::fmt::Display for CustomError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::error::Error for CustomError {}

pub fn fallible_constructor() -> Result<String, CustomError> {
    todo!()
}

pub fn error_handler(e: &pavex::Error) -> Response {
    todo!()
}

pub fn error_observer(e: &pavex::Error) {
    todo!()
}

pub fn handler(_s: String) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::fallible_constructor), Lifecycle::RequestScoped)
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
