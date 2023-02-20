use pavex_builder::{f, Blueprint, Lifecycle};
use pavex_runtime::response::Response;

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
    bp.route(f!(crate::handler), "/home");
    bp
}
