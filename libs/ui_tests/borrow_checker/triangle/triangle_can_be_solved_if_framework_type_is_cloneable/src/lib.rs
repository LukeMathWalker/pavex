use pavex::request::path::RawPathParams;
use pavex::Response;
use pavex::{blueprint::from, Blueprint};

// The call graph looks like this:
//
// RawPathParams
//  / \
// B   |&
//  \  |
// handler
//
// `RawPathParams` is a framework built-in type.
// `RawPathParams` cannot be borrowed by `handler` after it has been moved to construct `B`.
// `RawPathParams` is cloneable though!
// Pavex should detect this and clone `RawPathParams` before calling `B`'s constructor.

pub struct B;

#[pavex::request_scoped(id = "B_")]
pub fn b(_p: RawPathParams) -> B {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler(_p: &RawPathParams, _b: B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
