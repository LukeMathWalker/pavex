use pavex::blueprint::{from, Blueprint};
use pavex::response::Response;
use pavex::{f, t};

pub struct A;
pub struct B;
pub struct C;

pub fn singleton() -> A {
    todo!()
}

pub fn request_scoped() -> B {
    todo!()
}

pub struct A1;

#[pavex::singleton(clone_if_necessary)]
pub fn a1() -> A1 {
    todo!()
}

pub struct B1;

#[pavex::request_scoped(clone_if_necessary)]
pub fn b1() -> B1 {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.prebuilt(t!(crate::C)).clone_if_necessary();
    bp.singleton(f!(crate::singleton)).clone_if_necessary();
    bp.request_scoped(f!(crate::request_scoped))
        .clone_if_necessary();
    bp.routes(from![crate]);
    bp
}
