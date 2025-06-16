use pavex::blueprint::{from, Blueprint};
use pavex::http::StatusCode;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.nest(sub_blueprint());
    bp.route(PARENT);
    bp
}

#[pavex::singleton]
pub fn singleton() -> u64 {
    todo!()
}

#[pavex::request_scoped]
pub fn scoped() -> u32 {
    todo!()
}

#[pavex::transient]
pub fn transient() -> u16 {
    todo!()
}

#[pavex::get(path = "/parent")]
pub fn parent(_x: u64, _y: u32, _z: u16) -> StatusCode {
    todo!()
}

#[pavex::get(path = "/child")]
pub fn child(_x: u64, _y: u32, _z: u16) -> StatusCode {
    todo!()
}

fn sub_blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.route(CHILD);
    bp
}
