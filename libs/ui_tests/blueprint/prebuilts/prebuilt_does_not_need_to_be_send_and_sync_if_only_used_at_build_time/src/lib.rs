use pavex::blueprint::{from, Blueprint};
use pavex::response::Response;
use pavex::t;
use std::rc::Rc;

pub struct A(pub Rc<String>);

#[pavex::prebuilt]
#[derive(Clone)]
pub struct A1(pub Rc<String>);

#[derive(Clone)]
pub struct B;

#[pavex::singleton]
pub fn b(_a: A, _a1: A1) -> B {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_b: &B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.prebuilt(t!(crate::A));
    bp.routes(from![crate]);
    bp
}
