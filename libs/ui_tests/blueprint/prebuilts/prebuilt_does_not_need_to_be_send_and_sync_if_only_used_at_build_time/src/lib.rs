use pavex::blueprint::{from, router::GET, Blueprint};
use pavex::response::Response;
use pavex::{f, t};
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

pub fn handler(_b: &B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.prebuilt(t!(crate::A));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
