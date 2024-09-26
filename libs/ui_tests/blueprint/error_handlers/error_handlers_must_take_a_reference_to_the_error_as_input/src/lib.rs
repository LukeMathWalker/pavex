use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

#[derive(Debug)]
pub struct Error;

pub fn fallible_constructor() -> Result<String, Error> {
    todo!()
}

pub fn error_handler() -> Response {
    todo!()
}

pub fn handler(_s: String) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::fallible_constructor), Lifecycle::RequestScoped)
        .error_handler(f!(crate::error_handler));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
