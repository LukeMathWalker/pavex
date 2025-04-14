use pavex::blueprint::{from, router::GET, Blueprint};
use pavex::response::Response;
use pavex::{f, t};

// Not clonable.
pub struct A;

// Not clonable.
pub struct B;

// Not clonable.
#[pavex::config(key = "a1")]
pub struct A1;

// Not clonable.
// Should error even if marked as never clone.
#[pavex::config(key = "b1", never_clone)]
pub struct B1;

pub fn handler(_a: A, _b: B1) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.config("a", t!(crate::A));
    // It must generate an error even if the config is marked as never clone.
    bp.config("b", t!(crate::B)).never_clone();
    bp.route(GET, "/", f!(crate::handler));
    bp
}
