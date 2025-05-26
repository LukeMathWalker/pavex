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
// The type A is a singleton and therefore cloneable.
// Pavex should detect this and insert a `Clone` invocation.

#[derive(Clone)]
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
    // A is a singleton, so it will be an input parameter of the dependency closure for `handler`
    bp.singleton(f!(crate::a)).clone_if_necessary();
    bp.request_scoped(f!(crate::b));
    bp.request_scoped(f!(crate::c));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
