use std::sync::Arc;

use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::http::StatusCode;

pub struct Custom;

pub fn constructor() -> Arc<Custom> {
    Arc::new(Custom)
}

pub fn handler(_s: &Arc<Custom>) -> StatusCode {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.singleton(f!(crate::constructor));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
