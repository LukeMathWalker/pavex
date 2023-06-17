use std::path::PathBuf;

use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::http::StatusCode;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.nest(sub_blueprint());
    bp.route(GET, "/parent", f!(crate::handler));
    bp
}

pub fn singleton() -> u64 {
    todo!()
}

pub fn scoped() -> u32 {
    todo!()
}

pub fn transient() -> u16 {
    todo!()
}

pub fn handler(_x: u64, _y: u32, _z: u16) -> StatusCode {
    todo!()
}

fn sub_blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::singleton), Lifecycle::Singleton);
    bp.constructor(f!(crate::scoped), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::transient), Lifecycle::Transient);
    bp.route(GET, "/child", f!(crate::handler));
    bp
}
