use pavex::{blueprint::from, Blueprint};
use std::path::PathBuf;

#[pavex::request_scoped]
pub fn infallible_constructor() -> PathBuf {
    todo!()
}

#[derive(Debug)]
pub struct ExtractPathError;

#[pavex::error_handler]
pub fn error_handler(_e: &ExtractPathError) -> pavex::Response {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn request_handler(_inner: PathBuf) -> pavex::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(INFALLIBLE_CONSTRUCTOR)
        .error_handler(ERROR_HANDLER);
    bp.routes(from![crate]);
    bp
}
