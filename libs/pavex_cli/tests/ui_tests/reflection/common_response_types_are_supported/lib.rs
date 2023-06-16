use std::borrow::Cow;

use http::response::Parts;
use pavex::http::StatusCode;
use pavex::response::{Response, ResponseHead};
use pavex_builder::{constructor::Lifecycle, f, router::GET, Blueprint};

pub fn response() -> Response {
    todo!()
}

pub fn status_code() -> StatusCode {
    todo!()
}

pub fn parts() -> Parts {
    todo!()
}

pub fn response_head() -> ResponseHead {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/response", f!(crate::response));
    bp.route(GET, "/status_code", f!(crate::status_code));
    bp.route(GET, "/parts", f!(crate::parts));
    bp.route(GET, "/head", f!(crate::response_head));
    bp
}
