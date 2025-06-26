use pavex::response::Response;
use pavex::{blueprint::from, Blueprint};

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

// A is a singleton, so it will be an input parameter of the dependency closure for `handler`
#[pavex::singleton(id = "A_", clone_if_necessary)]
pub fn a() -> A {
    todo!()
}

#[pavex::request_scoped(id = "B_")]
pub fn b(_a: A) -> B {
    todo!()
}

#[pavex::request_scoped(id = "C_")]
pub fn c(_a: A) -> C {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler(_b: B, _c: C) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
