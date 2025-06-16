use pavex::blueprint::{from, Blueprint};
use pavex::response::Response;

// The call graph looks like this:
//
//    A
//  /    \
// |&mut  |&
// |      B<'_>
// |      |
// |      |
// handler
//
// `A` is not cloneable and:
// - it is borrowed by `B`, which holds a reference to `A` as one of its fields.
// - it is borrowed mutably by the handler.
//
// Pavex should detect that this graph can't satisfy the borrow checker
// (since `A` is not cloneable) and report an error.

pub struct A;

pub struct B<'a> {
    pub a: &'a A,
}

#[pavex::request_scoped(id = "A_")]
pub fn a() -> A {
    todo!()
}

#[pavex::request_scoped(id = "B_")]
pub fn b(_a: &A) -> B {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler(_a: &mut A, _b: B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
