use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;

pub fn handler() -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/home", f!(handler));
    bp
}
