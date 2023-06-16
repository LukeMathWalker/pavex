use std::path::PathBuf;

use pavex::extract::route::RawRouteParams;
use pavex::response::Response;
use pavex_builder::{
    constructor::{CloningStrategy, Lifecycle},
    f,
    router::GET,
    Blueprint,
};

// The call graph looks like this:
//
// RawRouteParams
//  / \
// B   C
//  \ /
// handler
//
// `RawRouteParams` is a framework-provided type that can be cloned if necessary.
// It is moved twice here.
// Pavex should detect this and insert a `Clone` invocation.

pub struct B;

pub struct C;

pub fn b(_p: RawRouteParams) -> B {
    todo!()
}

pub fn c(_p: RawRouteParams) -> C {
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
