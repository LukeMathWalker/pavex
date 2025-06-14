use pavex::blueprint::{constructor::Lifecycle, from, Blueprint};
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

#[pavex::get(path = "/home")]
pub fn handler(_s: String) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::fallible_constructor), Lifecycle::RequestScoped)
        .error_handler(f!(crate::error_handler));
    bp.routes(from![crate]);
    bp
}
