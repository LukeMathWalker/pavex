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
// The type `A` cannot be borrowed by `handler` after it has been moved to construct `B`.
// Pavex should detect this and report an error since `A` is not marked as `CloneIfNecessary`.

#[derive(Clone)]
pub struct A;

pub struct B;

#[pavex::singleton(id = "A_")]
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
