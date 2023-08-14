use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::middleware::Next;
use pavex::response::Response;

pub fn mw<T>(_next: Next<T>, _second_next: Next<T>) -> Response {
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
