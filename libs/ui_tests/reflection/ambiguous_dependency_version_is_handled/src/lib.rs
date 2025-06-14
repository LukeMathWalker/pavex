use pavex::blueprint::{from, Blueprint};
use pavex::http::StatusCode;

#[pavex::get(path = "/home")]
pub fn handler() -> StatusCode {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.routes(from![crate]);
    bp
}
