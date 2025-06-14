use pavex::{f, blueprint::{from, Blueprint}};
use pavex::response::Response;

// The call graph looks like this:
//
//   A
//  / \
// B   |&
//  \  |
// handler
//
// The type A is not cloneable, so it cannot be borrowed by `handler` after
// it has been moved to construct `B`.
// Pavex should detect this and report an error.

pub struct A;

pub struct B;

pub fn a() -> A {
    todo!()
}

pub fn b(_a: A) -> B {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler(_a: &A, _b: B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(crate::a));
    bp.request_scoped(f!(crate::b));
    bp.routes(from![crate]);
    bp
}
