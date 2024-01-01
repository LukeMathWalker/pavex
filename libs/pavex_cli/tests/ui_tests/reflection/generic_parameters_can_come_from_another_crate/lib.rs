use dep_1::Custom;
use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::response::IntoResponse;
use pavex::response::Response;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/home", f!(crate::handler));
    bp
}

// A locally-defined type
pub struct BodyType;

// The `Custom` type comes from a dependency but the body
// type is defined in this crate.
pub fn handler() -> Custom<BodyType> {
    todo!()
}
