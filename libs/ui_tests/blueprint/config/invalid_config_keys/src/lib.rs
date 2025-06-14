use pavex::blueprint::{from, Blueprint};
use pavex::response::Response;

#[derive(Clone)]
#[pavex::config(key = "2numbersfirst", id = "A_")]
pub struct A;

#[derive(Clone)]
#[pavex::config(key = "", id = "B_")]
pub struct B;

#[derive(Clone)]
#[pavex::config(key = "with-a-dash", id = "C_")]
pub struct C;

#[pavex::get(path = "/")]
pub fn handler(_a: A, _b: B, _c: C) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
