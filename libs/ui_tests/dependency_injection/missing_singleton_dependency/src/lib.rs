use pavex::Response;
use pavex::{blueprint::from, Blueprint};

#[derive(Clone)]
pub struct A;

#[derive(Clone)]
pub struct B;

#[pavex::singleton(id = "B_")]
pub fn b(_a: &A) -> B {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_b: &B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
