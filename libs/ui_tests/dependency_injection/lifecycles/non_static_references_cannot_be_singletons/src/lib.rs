use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::http::StatusCode;

#[derive(Clone)]
pub struct A;

#[derive(Clone)]
pub struct B;

pub fn a() -> A {
    todo!()
}

pub fn b(a: &A) -> &B {
    todo!()
}

pub fn handler(_b: &B) -> StatusCode {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::a), Lifecycle::Singleton);
    bp.constructor(f!(crate::b), Lifecycle::Singleton);
    bp.route(GET, "/", f!(crate::handler));
    bp
}
