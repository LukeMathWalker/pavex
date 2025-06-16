use pavex::blueprint::{from, Blueprint};
use pavex::request::RequestHead;
use pavex::response::Response;

// The call graph looks like this:
//
// Request
//  / \
// B   C
//  \ /
// handler
//
// `Request` is a framework-provided type that cannot be cloned.
// It is moved twice here.
// Pavex should detect this and report an error.

pub struct B;

pub struct C;

#[pavex::request_scoped(id = "B_")]
pub fn b(_p: RequestHead) -> B {
    todo!()
}

#[pavex::request_scoped(id = "C_")]
pub fn c(_p: RequestHead) -> C {
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
