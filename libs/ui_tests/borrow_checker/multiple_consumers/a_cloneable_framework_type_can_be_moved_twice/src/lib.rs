use pavex::request::path::RawPathParams;
use pavex::response::Response;
use pavex::{
    blueprint::{from, Blueprint},
    f,
};

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

pub fn b(_p: RawPathParams) -> B {
    todo!()
}

pub fn c(_p: RawPathParams) -> C {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler(_b: B, _c: C) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(crate::b));
    bp.request_scoped(f!(crate::c));
    bp.routes(from![crate]);
    bp
}
