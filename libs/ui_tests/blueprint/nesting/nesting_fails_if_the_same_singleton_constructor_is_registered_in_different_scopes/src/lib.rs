use pavex::blueprint::{constructor::Lifecycle, Blueprint};
use pavex::f;
use pavex::http::StatusCode;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::singleton), Lifecycle::Singleton);
    bp.route(PARENT_HANDLER);
    bp.nest(sub_blueprint());
    bp
}

pub fn singleton() -> u64 {
    todo!()
}

#[pavex::get(path = "/parent")]
pub fn parent_handler(_x: u64) -> StatusCode {
    todo!()
}

#[pavex::get(path = "/child")]
pub fn child_handler(_x: u64) -> StatusCode {
    todo!()
}

fn sub_blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::singleton), Lifecycle::Singleton);
    bp.route(CHILD_HANDLER);
    bp
}
