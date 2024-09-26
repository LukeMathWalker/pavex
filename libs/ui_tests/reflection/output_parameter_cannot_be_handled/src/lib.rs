use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;

pub fn c() -> Box<dyn std::error::Error> {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/home", f!(crate::c));
    bp
}
