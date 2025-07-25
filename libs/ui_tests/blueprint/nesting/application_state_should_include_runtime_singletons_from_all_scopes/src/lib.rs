use pavex::http::StatusCode;
use pavex::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(PARENT_SINGLETON);
    bp.route(PARENT_HANDLER);
    bp.nest(sub_blueprint());
    bp
}

#[pavex::get(path = "/parent")]
pub fn parent_handler(_x: u64) -> StatusCode {
    todo!()
}

#[pavex::singleton]
pub fn parent_singleton() -> u64 {
    todo!()
}

fn sub_blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(SINGLETON_DEP);
    bp.constructor(NESTED_SINGLETON);
    bp.route(CHILD_HANDLER);
    bp
}

#[pavex::singleton]
pub fn singleton_dep() -> u16 {
    todo!()
}

#[pavex::singleton]
pub fn nested_singleton(_x: u16) -> u32 {
    todo!()
}

#[pavex::get(path = "/child")]
pub fn child_handler(_x: u32) -> StatusCode {
    todo!()
}
