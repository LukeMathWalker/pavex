use std::path::PathBuf;

use pavex::blueprint::{
    constructor::{CloningStrategy, Lifecycle},
    router::GET,
    Blueprint,
};
use pavex::f;
use pavex::request::path::RawPathParams;
use pavex::response::Response;

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
