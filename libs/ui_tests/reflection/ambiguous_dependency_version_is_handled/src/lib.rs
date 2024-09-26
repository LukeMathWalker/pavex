use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::http::StatusCode;

pub fn handler() -> StatusCode {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
