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
// The type A is not cloneable, so it cannot be moved twice.
// Pavex should detect this and report an error.

pub struct A;

pub struct B;

pub struct C;

pub fn a() -> A {
    todo!()
}

pub fn b(_a: A) -> B {
    todo!()
}

pub fn c(_a: A) -> C {
    todo!()
}

pub fn handler(_b: B, _c: C) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(crate::a));
    bp.request_scoped(f!(crate::b));
    bp.request_scoped(f!(crate::c));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
