use pavex::blueprint::{from, Blueprint};
use pavex::response::Response;
use std::rc::Rc;

#[derive(Debug, Clone, serde::Deserialize)]
#[pavex::config(key = "a", id = "CONFIG_A")]
// Only used before starting to serve requests.
pub struct A(pub Rc<String>);

#[derive(Debug, Clone)]
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
