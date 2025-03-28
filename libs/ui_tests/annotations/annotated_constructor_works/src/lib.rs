use pavex::blueprint::{from, router::GET, Blueprint};
use pavex::f;

pub struct A;

#[pavex::constructor(lifecycle = "request_scoped")]
pub fn new() -> A {
    A
}

pub fn handler(_x: &A) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.route(GET, "/handler", f!(crate::handler));
    bp
}
