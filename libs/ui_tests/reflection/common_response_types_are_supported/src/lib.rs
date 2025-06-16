use http::response::Parts;
use pavex::blueprint::{from, Blueprint};
use pavex::http::StatusCode;
use pavex::response::{Response, ResponseHead};

#[pavex::get(path = "/response")]
pub fn route_response() -> Response {
    todo!()
}

#[pavex::get(path = "/status_code")]
pub fn route_status_code() -> StatusCode {
    todo!()
}

#[pavex::get(path = "/parts")]
pub fn route_parts() -> Parts {
    todo!()
}

#[pavex::get(path = "/head")]
pub fn route_response_head() -> ResponseHead {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.routes(from![crate]);
    bp
}
