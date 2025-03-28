use pavex::blueprint::{from, router::GET, Blueprint};
use pavex::f;

#[derive(Clone)]
pub struct A;

#[pavex::singleton(cloning_strategy = "clone_if_necessary")]
pub fn a() -> A {
    A
}

pub struct B<T>(T);

#[pavex::request_scoped]
pub fn b<T>(_i: T) -> B<T> {
    todo!()
}

pub fn handler(_x: &A, _y: &B<A>) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.route(GET, "/handler", f!(crate::handler));
    bp
}
