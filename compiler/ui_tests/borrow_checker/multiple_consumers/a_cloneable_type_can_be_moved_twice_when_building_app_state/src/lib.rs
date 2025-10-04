use pavex::Response;
use pavex::{blueprint::from, Blueprint};

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

#[pavex::singleton(id = "A_", clone_if_necessary)]
pub fn a() -> Result<A, pavex::Error> {
    todo!()
}

#[pavex::singleton(id = "B_")]
pub fn b(_a: A) -> B {
    todo!()
}

#[pavex::singleton(id = "C_")]
pub fn c(_a: A) -> Result<C, pavex::Error> {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_b: &C, _c: &B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
