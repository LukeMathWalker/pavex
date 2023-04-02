use std::path::PathBuf;

use pavex_builder::{constructor::Lifecycle, f, router::GET, Blueprint};

pub fn infallible_constructor() -> PathBuf {
    todo!()
}

#[derive(Debug)]
pub struct ExtractPathError;

pub fn error_handler(_e: &ExtractPathError) -> pavex_runtime::response::Response {
    todo!()
}

pub fn request_handler(_inner: PathBuf) -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::infallible_constructor), Lifecycle::RequestScoped)
        .error_handler(f!(crate::error_handler));
    bp.route(GET, "/home", f!(crate::request_handler));
    bp
}
