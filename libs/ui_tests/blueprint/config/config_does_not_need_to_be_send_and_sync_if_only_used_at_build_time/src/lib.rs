use pavex::blueprint::{from, router::GET, Blueprint};
use pavex::response::Response;
use pavex::{f, t};
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
pub fn b(_a: &A) -> B {
    todo!()
}

pub fn handler(_b: &B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.config("a", t!(crate::A));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
