use pavex_builder::{Blueprint, router::GET, f};
use pavex_runtime::hyper::StatusCode;

pub fn ping() -> StatusCode {
    StatusCode::OK
}

pub fn app_blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/api/ping", f!(crate::ping));
    bp
}