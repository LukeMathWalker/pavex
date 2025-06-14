use pavex::blueprint::{from, Blueprint};
use pavex::response::Response;
use pavex::{f, t};

#[derive(Clone)]
pub struct A;

#[derive(Clone)]
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
    use pavex::response::Response;

    #[derive(Clone)]
    pub struct A;

    #[derive(Clone)]
    pub struct B;

    #[pavex::singleton]
    pub fn a() -> A {
        todo!()
    }

    #[pavex::singleton]
    pub fn b(_b: A) -> B {
        todo!()
    }

    #[pavex::get(path = "/annotated")]
    pub fn handler(_a: A, _b: B) -> Response {
        todo!()
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from!(crate::annotated));
    bp.prebuilt(t!(crate::B));
    bp.singleton(f!(crate::singleton));
    bp.singleton(f!(crate::singleton2));
    bp.routes(from![crate]);
    bp
}
