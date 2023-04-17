use std::path::PathBuf;

use pavex_builder::{constructor::{Lifecycle, CloningStrategy}, f, router::GET, Blueprint};
use pavex_runtime::response::Response;
use pavex_runtime::http::Request;
use pavex_runtime::hyper::Body;

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

pub fn b(_p: Request<Body>) -> B {
    todo!()
}

pub fn c(_p: Request<Body>) -> C {
    todo!()
}

pub fn handler(_b: B, _c: C) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::b), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::c), Lifecycle::RequestScoped);
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
