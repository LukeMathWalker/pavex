use pavex::Response;
use pavex::{blueprint::from, Blueprint};

// The call graph looks like this:
//
//   A
//  / \
// B   |&
//  \  |
// handler
//
// `A` cannot be borrowed by `handler` after it has been moved to construct `B`.
// `A` is cloneable though!
// Pavex should detect this and clone `A` before calling `B`'s constructor.

#[derive(Clone)]
pub struct A;

pub struct B;

#[pavex::request_scoped(id = "A_", clone_if_necessary)]
pub fn a() -> A {
    todo!()
}

#[pavex::request_scoped(id = "B_")]
pub fn b(_a: A) -> B {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler(_a: &A, _b: B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
