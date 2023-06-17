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
//   A
//  / \
// B   |&
//  \  |
// handler
//
// `A` cannot be borrowed by `handler` after it has been moved to construct `B`.
// `A` is `Copy` though!
// Pavex should detect this and accept the graph as is.

#[derive(Clone, Copy)]
pub struct A;

pub struct B;

pub fn a() -> A {
    todo!()
}

pub fn b(_a: A) -> B {
    todo!()
}

pub fn handler(_a: &A, _b: B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::a), Lifecycle::RequestScoped)
        .cloning(CloningStrategy::CloneIfNecessary);
    bp.constructor(f!(crate::b), Lifecycle::RequestScoped);
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
