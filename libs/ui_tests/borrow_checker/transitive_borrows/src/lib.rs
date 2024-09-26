use std::path::PathBuf;

use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

// The call graph looks like this:
//
//   A
//  / \
// B   |&
// |  C<'_>
// |   |
// handler
//
// `A` is not clonable and:
// - it is consumed by `B`;
// - it is borrowed by `C`, which holds a reference to `A` as one of its fields.
//
// Pavex should detect that this graph can't satisfy the borrow checker (since `A` is not clonable) and report an error.

pub struct A;

pub struct B;

pub struct C<'a> {
    a: &'a A,
}

pub fn a() -> A {
    todo!()
}

pub fn b(_a: A) -> B {
    todo!()
}

pub fn c(_a: &A) -> C {
    todo!()
}

pub fn handler(_c: C, _b: B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::a), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::b), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::c), Lifecycle::RequestScoped);
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
