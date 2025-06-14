use pavex::blueprint::{from, Blueprint};
use pavex::response::Response;
use pavex::t;
use std::rc::Rc;

#[derive(Debug, Clone, serde::Deserialize)]
// Only used before starting to serve requests.
pub struct A(pub Rc<String>);

#[derive(Debug, Clone, serde::Deserialize)]
#[pavex::config(key = "a1")]
// Only used before starting to serve requests.
pub struct A1(pub Rc<String>);

#[derive(Debug, Clone, serde::Deserialize)]
pub struct B;

#[pavex::singleton]
pub fn b(_a: &A, _a1: &A1) -> B {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_b: &B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.config("a", t!(crate::A));
    bp.routes(from![crate]);
    bp
}
