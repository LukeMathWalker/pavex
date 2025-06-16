use pavex::blueprint::{from, Blueprint};
use pavex::response::Response;

#[derive(Clone)]
pub struct A;

#[derive(Clone)]
#[pavex::prebuilt(id = "B_")]
pub struct B;

#[derive(Clone)]
pub struct C;

#[pavex::singleton(id = "A_")]
pub fn a() -> A {
    todo!()
}

#[pavex::singleton(id = "C_")]
pub fn c(_b: B) -> C {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_a: A, _b: B, _c: C) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from!(crate));
    bp.routes(from![crate]);
    bp
}
