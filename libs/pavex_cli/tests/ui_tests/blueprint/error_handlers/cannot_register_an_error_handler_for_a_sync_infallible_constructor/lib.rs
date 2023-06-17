use std::path::PathBuf;

use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;

pub fn infallible_constructor() -> PathBuf {
    todo!()
}

#[derive(Debug)]
pub struct ExtractPathError;

pub fn error_handler(_e: &ExtractPathError) -> pavex::response::Response {
    todo!()
}

pub fn request_handler(_inner: PathBuf) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::infallible_constructor), Lifecycle::RequestScoped)
        .error_handler(f!(crate::error_handler));
    bp.route(GET, "/home", f!(crate::request_handler));
    bp
}
