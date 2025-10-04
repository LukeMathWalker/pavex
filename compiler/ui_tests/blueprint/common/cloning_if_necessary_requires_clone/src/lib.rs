use pavex::Response;
use pavex::{blueprint::from, Blueprint};

pub struct A;

#[pavex::singleton(clone_if_necessary, id = "A_")]
pub fn a() -> A {
    todo!()
}

pub struct B;

#[pavex::request_scoped(clone_if_necessary, id = "B_")]
pub fn b() -> B {
    todo!()
}

#[pavex::prebuilt(clone_if_necessary, id = "C_")]
pub struct C;

#[pavex::get(path = "/")]
pub fn handler() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
