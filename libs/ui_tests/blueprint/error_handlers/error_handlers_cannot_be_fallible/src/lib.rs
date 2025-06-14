use pavex::blueprint::{from, Blueprint};
use pavex::response::Response;

#[pavex::error_handler]
pub fn error_handler(_e: &pavex::Error) -> Result<Response, String> {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler() -> Result<Response, pavex::Error> {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
