use pavex::response::Response;
use pavex::{blueprint::from, Blueprint};

// The call graph looks like this:
//
//  A   B
// &| X |&
//  D   C
//   \ /
// handler
//
// If `D` is constructed before `C`, then `A` cannot be borrowed by `C`'s constructor after it
// has been moved to construct `D`.
// If `C` is constructed before `D`, then `B` cannot be borrowed by `D`'s constructor after it
// has been moved to construct `C`.
//
// Both `A` and `B` are `Copy` though!
// Pavex should detect this accept the graph as is.

#[derive(Clone, Copy)]
pub struct A;

#[derive(Clone, Copy)]
pub struct B;

pub struct C;

pub struct D;

#[pavex::request_scoped(id = "A_", clone_if_necessary)]
pub fn a() -> A {
    todo!()
}

#[pavex::request_scoped(id = "B_", clone_if_necessary)]
pub fn b() -> B {
    todo!()
}

#[pavex::request_scoped(id = "C_")]
pub fn c(_a: A, _b: &B) -> C {
    todo!()
}

#[pavex::request_scoped(id = "D_")]
pub fn d(_a: &A, _b: B) -> D {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler(_c: C, _d: D) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
