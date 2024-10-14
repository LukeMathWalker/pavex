use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::request::path::RawPathParams;
use pavex::response::Response;

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

pub fn b(_p: RawPathParams) -> B {
    todo!()
}

pub fn handler(_p: &RawPathParams, _b: B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(crate::b));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
