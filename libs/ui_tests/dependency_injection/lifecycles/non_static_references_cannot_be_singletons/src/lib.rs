use pavex::blueprint::{from, Blueprint};
use pavex::http::StatusCode;

#[derive(Clone)]
pub struct A;

#[derive(Clone)]
pub struct B;

#[pavex::singleton(id = "A_")]
pub fn a() -> A {
    todo!()
}

#[pavex::singleton(id = "B_")]
pub fn b(_a: &A) -> &B {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_b: &B) -> StatusCode {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
