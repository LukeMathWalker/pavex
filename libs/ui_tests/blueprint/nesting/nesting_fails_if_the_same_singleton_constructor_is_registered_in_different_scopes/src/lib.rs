use std::path::PathBuf;

use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::http::StatusCode;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::singleton), Lifecycle::Singleton);
    bp.route(GET, "/parent", f!(crate::handler));
    bp.nest(sub_blueprint());
    bp
}

pub fn singleton() -> u64 {
    todo!()
}

pub fn handler(_x: u64) -> StatusCode {
    todo!()
}

fn sub_blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::singleton), Lifecycle::Singleton);
    bp.route(GET, "/child", f!(crate::handler));
    bp
}
