use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

pub fn mw() -> Response {
    todo!()
}

pub fn handler() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.post_process(f!(crate::mw));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
