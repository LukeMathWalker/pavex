use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

// The call graph looks like this:
//
//   A
//  / \
// B   C
//  \ /
// handler
//
// The type A is cloneable.
// Pavex should detect this and insert a Clone call.

#[derive(Clone)]
pub struct A;

pub struct B;

pub struct C;

pub fn a() -> Result<A, pavex::Error> {
    todo!()
}

pub fn b(_a: A) -> B {
    todo!()
}

pub fn c(_a: A) -> Result<C, pavex::Error> {
    todo!()
}

pub fn handler(_b: &C, _c: &B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.singleton(f!(crate::a)).clone_if_necessary();
    bp.singleton(f!(crate::b));
    bp.singleton(f!(crate::c));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
