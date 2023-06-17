use std::path::PathBuf;

use pavex::f;
use pavex::blueprint::{
    constructor::{CloningStrategy, Lifecycle},
    router::GET,
    Blueprint,
};
use pavex::response::Response;

// The call graph looks like this:
//
//  A   B
// &| X |&
//  D   C
//   \ /
// handler
//
// If `D` is constructed before `C`, then `A` cannot be borrowed by `C`'s constructor after it
// has been moved to construct `D`.
// If `C` is constructed before `D`, then `B` cannot be borrowed by `D`'s constructor after it
// has been moved to construct `C`.
//
// Both `A` and `B` are `Copy` though!
// Pavex should detect this accept the graph as is.

#[derive(Clone, Copy)]
pub struct A;

#[derive(Clone, Copy)]
pub struct B;

pub struct C;

pub struct D;

pub fn a() -> A {
    todo!()
}

pub fn b() -> B {
    todo!()
}

pub fn c(_a: A, _b: &B) -> C {
    todo!()
}

pub fn d(_a: &A, _b: B) -> D {
    todo!()
}

pub fn handler(_c: C, _d: D) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::a), Lifecycle::RequestScoped)
        .cloning(CloningStrategy::CloneIfNecessary);
    bp.constructor(f!(crate::b), Lifecycle::RequestScoped)
        .cloning(CloningStrategy::CloneIfNecessary);
    bp.constructor(f!(crate::c), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::d), Lifecycle::RequestScoped);
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
