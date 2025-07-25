use pavex::Response;
use pavex::{blueprint::from, Blueprint};

// The call graph looks like this:
//
//   A
//  / \
// B   |&
// |  C<'_>
// |   |
// handler
//
// `A` is not cloneable and:
// - it is consumed by `B`;
// - it is borrowed by `C`, which holds a reference to `A` as one of its fields.
//
// Pavex should detect that this graph can't satisfy the borrow checker (since `A` is not cloneable) and report an error.

pub struct A;

pub struct B;

pub struct C<'a> {
    pub a: &'a A,
}

#[pavex::request_scoped(id = "A_")]
pub fn a() -> A {
    todo!()
}

#[pavex::request_scoped(id = "B_")]
pub fn b(_a: A) -> B {
    todo!()
}

#[pavex::request_scoped(id = "C_")]
pub fn c(_a: &A) -> C {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler(_c: C, _b: B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
