use pavex::http::StatusCode;
use pavex::{blueprint::from, Blueprint};

#[derive(Clone)]
pub struct A;

pub struct B;

pub struct C;

#[pavex::singleton(id = "A_")]
pub fn a(_b: B, _c: C) -> A {
    todo!()
}

#[pavex::request_scoped(id = "B_")]
pub fn b() -> B {
    todo!()
}

#[pavex::transient(id = "C_")]
pub fn c() -> C {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_a: &A) -> StatusCode {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
