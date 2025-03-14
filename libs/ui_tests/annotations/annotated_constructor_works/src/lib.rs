use pavex::blueprint::{from, router::GET, Blueprint};
use pavex::f;

pub struct A;

impl A {
    #[pavex::constructor(lifecycle = "request_scoped")]
    pub fn new() -> Self {
        Self
    }
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
