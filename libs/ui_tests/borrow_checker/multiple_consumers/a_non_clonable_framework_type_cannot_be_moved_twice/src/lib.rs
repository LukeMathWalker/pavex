use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
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

pub fn b(_p: RequestHead) -> B {
    todo!()
}

pub fn c(_p: RequestHead) -> C {
    todo!()
}

pub fn handler(_b: B, _c: C) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(crate::b));
    bp.request_scoped(f!(crate::c));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
