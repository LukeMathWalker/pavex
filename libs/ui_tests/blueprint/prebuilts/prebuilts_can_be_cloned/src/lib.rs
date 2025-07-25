use pavex::Response;
use pavex::{blueprint::from, Blueprint};

#[derive(Clone)]
#[pavex::prebuilt(clone_if_necessary, id = "A_")]
pub struct A;

#[derive(Clone)]
pub struct B;

#[pavex::singleton(id = "B_")]
// Consumes `A` by value`, but `A` is also needed
// as an input parameter to the request handler,
// thus the need to clone.
pub fn b(_a: A) -> B {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_a: A, _b: &B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
