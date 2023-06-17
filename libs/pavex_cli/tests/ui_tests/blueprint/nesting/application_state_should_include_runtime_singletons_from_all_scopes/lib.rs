use std::path::PathBuf;

use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::http::StatusCode;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::parent_singleton), Lifecycle::Singleton);
    bp.route(GET, "/parent", f!(crate::parent_handler));
    bp.nest(sub_blueprint());
    bp
}

pub fn parent_handler(_x: u64) -> StatusCode {
    todo!()
}

pub fn parent_singleton() -> u64 {
    todo!()
}

fn sub_blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::singleton_dep), Lifecycle::Singleton);
    bp.constructor(f!(crate::nested_singleton), Lifecycle::Singleton);
    bp.route(GET, "/child", f!(crate::nested_handler));
    bp
}

pub fn singleton_dep() -> u16 {
    todo!()
}

pub fn nested_singleton(_x: u16) -> u32 {
    todo!()
}

pub fn nested_handler(_x: u32) -> StatusCode {
    todo!()
}
