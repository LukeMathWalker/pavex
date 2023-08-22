use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::middleware::Next;
use pavex::response::Response;

pub struct Custom<T>(T);

pub fn mw<T>(_next: Next<Custom<T>>) -> Response {
    todo!()
}

pub fn handler() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(f!(crate::mw));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
