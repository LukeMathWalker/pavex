use pavex::http::StatusCode;
use pavex::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(SINGLETON);
    bp.route(PARENT_HANDLER);
    bp.nest(sub_blueprint());
    bp
}

#[pavex::singleton]
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
    bp.constructor(SINGLETON);
    bp.route(CHILD_HANDLER);
    bp
}
