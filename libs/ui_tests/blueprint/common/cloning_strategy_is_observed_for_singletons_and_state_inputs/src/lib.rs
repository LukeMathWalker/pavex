use pavex::blueprint::{constructor::CloningStrategy, router::GET, Blueprint};
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

pub fn singleton2(b: B) -> C {
    todo!()
}

pub fn handler(a: A, b: B, c: C) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.prebuilt(t!(crate::B));
    bp.singleton(f!(crate::singleton));
    bp.singleton(f!(crate::singleton2));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
