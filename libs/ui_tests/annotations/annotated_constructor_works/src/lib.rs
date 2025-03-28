use pavex::blueprint::{from, router::GET, Blueprint};
use pavex::f;

#[derive(Clone)]
pub struct A;

#[pavex::singleton(clone_if_necessary)]
/// As simple as it gets.
pub fn a() -> A {
    A
}

pub struct B<T>(T);

#[pavex::request_scoped]
/// Generic, but all generic parameters are used in the output type.
pub fn b<T>(_i: T) -> B<T> {
    todo!()
}

pub struct C;

#[pavex::transient(error_handler = "crate::error_handler")]
/// Fallible.
pub fn c(_b: &B<A>) -> Result<C, pavex::Error> {
    todo!()
}

pub struct D<'a> {
    _c: &'a C,
    _a: &'a A,
}

#[pavex::constructor(transient)]
/// With a lifetime parameter.
pub fn d<'a>(_c: &'a C, _a: &'a A) -> D<'a> {
    todo!()
}

pub fn error_handler(_error: &pavex::Error) -> pavex::response::Response {
    todo!()
}

pub fn handler(_x: &A, _y: &B<A>, _d: &D) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.route(GET, "/handler", f!(crate::handler));
    bp
}
