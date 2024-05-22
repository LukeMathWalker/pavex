use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

#[derive(Clone)]
pub struct A;

#[derive(Clone)]
pub struct B;

pub fn b(a: &A) -> B {
    todo!()
}

pub fn handler(_b: &B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.singleton(f!(crate::b));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
