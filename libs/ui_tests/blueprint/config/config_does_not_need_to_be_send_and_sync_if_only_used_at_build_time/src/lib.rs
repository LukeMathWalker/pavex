use pavex::blueprint::{router::GET, Blueprint};
use pavex::response::Response;
use pavex::{f, t};
use std::rc::Rc;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct A(pub Rc<String>);

#[derive(Debug, Clone, serde::Deserialize)]
pub struct B;

pub fn b(_a: &A) -> B {
    todo!()
}

pub fn handler(_b: &B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.config("a", t!(crate::A));
    bp.singleton(f!(crate::b));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
