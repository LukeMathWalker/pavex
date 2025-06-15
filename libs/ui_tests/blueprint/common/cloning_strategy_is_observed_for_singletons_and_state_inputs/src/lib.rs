use pavex::blueprint::{from, Blueprint};
use pavex::f;
use pavex::response::Response;

#[derive(Clone)]
pub struct A;

#[derive(Clone)]
#[pavex::prebuilt(id = "B_")]
pub struct B;

#[derive(Clone)]
pub struct C;

pub fn singleton() -> A {
    todo!()
}

pub fn singleton2(_b: B) -> C {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_a: A, _b: B, _c: C) -> Response {
    todo!()
}

pub mod annotated {
    use super::B;
    use pavex::response::Response;

    #[derive(Clone)]
    pub struct A;

    #[pavex::singleton]
    pub fn a() -> A {
        todo!()
    }

    #[pavex::get(path = "/annotated")]
    pub fn handler(_a: A, _b: B) -> Response {
        todo!()
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from!(crate));
    bp.singleton(f!(crate::singleton));
    bp.singleton(f!(crate::singleton2));
    bp.routes(from![crate]);
    bp
}
