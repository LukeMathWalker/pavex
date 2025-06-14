use pavex::blueprint::{from, Blueprint};
use pavex::f;
use pavex::response::Response;

#[derive(Clone)]
pub struct A;

#[derive(Clone)]
pub struct B;

pub fn b(_a: &A) -> B {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_b: &B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.singleton(f!(crate::b));
    bp.routes(from![crate]);
    bp
}
