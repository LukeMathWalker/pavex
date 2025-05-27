use pavex::blueprint::{from, router::GET, Blueprint};
use pavex::response::Response;

use pavex::f;

#[derive(Copy, Clone)]
pub struct A;

impl A {
    pub fn new() -> Self {
        Self
    }
}

pub fn a() -> A {
    todo!()
}

#[derive(Copy, Clone)]
pub struct B;

pub fn b() -> B {
    todo!()
}

pub fn handler_1(_a: A, _b: B) -> Response {
    todo!()
}

#[derive(Copy, Clone)]
pub struct A1;

#[pavex::singleton(never_clone)]
pub fn a1() -> A1 {
    todo!()
}

#[derive(Copy, Clone)]
pub struct B1;

#[pavex::singleton(never_clone)]
pub fn b1() -> B1 {
    todo!()
}

pub fn handler_2(_a1: A1, _b1: B1) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.singleton(f!(crate::b)).never_clone();
    bp.request_scoped(f!(crate::a)).never_clone();
    bp.route(GET, "/", f!(crate::handler_1));
    bp.route(GET, "/2", f!(crate::handler_2));
    bp
}
