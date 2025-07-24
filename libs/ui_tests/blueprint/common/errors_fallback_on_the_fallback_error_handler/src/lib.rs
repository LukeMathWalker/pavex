use pavex::blueprint::from;
use pavex::middleware::Processing;
use pavex::response::Response;
use pavex::Blueprint;
use pavex::{get, methods, pre_process};

#[pre_process(allow(error_fallback))]
pub fn pre() -> Result<Processing, CustomError> {
    todo!()
}

pub struct Dep;

#[methods]
impl Dep {
    #[request_scoped(allow(error_fallback))]
    pub fn new() -> Result<Self, CustomError> {
        todo!()
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Unhandled error")]
pub struct CustomError;

#[get(path = "/")]
// No allow, this will trigger a warning.
pub fn handler(_dep: Dep) -> Result<Response, CustomError> {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    // Even though we don't import from `pavex`, we still get the default
    // fallback error handler.
    bp.import(from![crate]);
    bp.pre_process(PRE);
    bp.routes(from![crate]);
    bp
}
