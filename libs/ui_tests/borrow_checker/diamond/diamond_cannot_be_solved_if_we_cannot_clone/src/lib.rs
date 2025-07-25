use pavex::Response;
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
// Pavex should detect this and return two errors.

pub struct A;

pub struct B;

pub struct C;

pub struct D;

#[pavex::request_scoped(id = "A_")]
pub fn a() -> A {
    todo!()
}

// Being a singleton, this will be an input type of the dependency closure for the request handler
#[pavex::singleton(id = "B_")]
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
