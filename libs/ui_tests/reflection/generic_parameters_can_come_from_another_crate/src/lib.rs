use dep_1::Custom;
use pavex::blueprint::{from, Blueprint};

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.routes(from![crate]);
    bp
}

// A locally-defined type
pub struct BodyType;

// The `Custom` type comes from a dependency but the body
// type is defined in this crate.
#[pavex::get(path = "/home")]
pub fn handler() -> Custom<BodyType> {
    todo!()
}
