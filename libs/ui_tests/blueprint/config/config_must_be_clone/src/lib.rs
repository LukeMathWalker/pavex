use pavex::blueprint::{router::GET, Blueprint};
use pavex::response::Response;
use pavex::{f, t};

// Not clonable.
pub struct A;

// Not clonable.
pub struct B;

pub fn handler(_a: A) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.config("a", t!(crate::A));
    // It must generate an error even if the config is marked as never clone.
    bp.config("b", t!(crate::B)).never_clone();
    bp.route(GET, "/", f!(crate::handler));
    bp
}
