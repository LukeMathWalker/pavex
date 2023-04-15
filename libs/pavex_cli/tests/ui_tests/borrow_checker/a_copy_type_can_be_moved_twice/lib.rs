use std::path::PathBuf;

use pavex_builder::{
    constructor::{CloningStrategy, Lifecycle},
    f,
    router::GET,
    Blueprint,
};
use pavex_runtime::response::Response;

// The call graph looks like this:
//
//   A
//  / \
// B   C
//  \ /
// handler
//
// The type A implements Copy.
// Pavex should detect this and accept the call graph as is.

#[derive(Copy, Clone)]
pub struct A;

pub struct B;

pub struct C;

pub fn a() -> A {
    todo!()
}

pub fn b(_a: A) -> B {
    todo!()
}

pub fn c(_a: A) -> C {
    todo!()
}

pub fn handler(_b: B, _c: C) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::a), Lifecycle::RequestScoped)
        .cloning(CloningStrategy::CloneIfNecessary);
    bp.constructor(f!(crate::b), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::c), Lifecycle::RequestScoped);
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
