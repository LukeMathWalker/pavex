use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;

pub struct A;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/", f!(crate::A));
    bp
}
