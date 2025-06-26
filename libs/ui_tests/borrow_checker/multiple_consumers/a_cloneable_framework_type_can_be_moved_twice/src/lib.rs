use pavex::request::path::RawPathParams;
use pavex::response::Response;
use pavex::{blueprint::from, Blueprint};

// The call graph looks like this:
//
// RawPathParams
//  / \
// B   C
//  \ /
// handler
//
// `RawPathParams` is a framework-provided type that can be cloned if necessary.
// It is moved twice here.
// Pavex should detect this and insert a `Clone` invocation.

pub struct B;

pub struct C;

#[pavex::request_scoped(id = "B_")]
pub fn b(_p: RawPathParams) -> B {
    todo!()
}

#[pavex::request_scoped(id = "C_")]
pub fn c(_p: RawPathParams) -> C {
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
