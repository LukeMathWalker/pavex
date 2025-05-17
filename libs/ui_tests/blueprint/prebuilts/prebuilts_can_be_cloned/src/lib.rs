use pavex::blueprint::{from, router::GET, Blueprint};
use pavex::response::Response;
use pavex::{f, t};

#[derive(Clone)]
pub struct A;

#[derive(Clone)]
#[pavex::prebuilt(clone_if_necessary)]
pub struct A1;

#[derive(Clone)]
pub struct B;

#[pavex::singleton]
pub fn b(_a: A, _a1: A1) -> B {
    todo!()
}

pub fn handler(_a: A, _b: &B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.prebuilt(t!(crate::A)).clone_if_necessary();
    bp.route(GET, "/", f!(crate::handler));
    bp
}
