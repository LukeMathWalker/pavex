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
//   Result<A, E>
//       |
//     match
//    /     \
//   /       \
// Ok(A)  B   Err(E)
//  |   / ||    |
//  |  /  ||    |
//  | /   ||    |
//  C    /  \   |
//  |   /    \  |
// handler   error h
//
// The type B is not cloneable and has three consumers that take it by value (`C`, `handler` and `error handler`).
// `error handler` is the only consumer in its control flow branch, so it's fine.
// `handler `and `C` are in the same control flow branch, so that can't work since they both consume `B`
// and `B` is not cloneable.
//
// Pavex's error should not mention `error handler`.

pub struct A;

#[derive(Debug)]
pub struct Error;

pub struct B;

pub struct C;

pub fn a() -> Result<A, Error> {
    todo!()
}

pub fn error_handler(_e: &Error, _b: B) -> Response {
    todo!()
}

pub fn b() -> B {
    todo!()
}

pub fn c(_a: A, _b: B) -> C {
    todo!()
}

pub fn handler(_b: B, _c: C) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::a), Lifecycle::RequestScoped)
        .error_handler(f!(crate::error_handler));
    bp.constructor(f!(crate::b), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::c), Lifecycle::RequestScoped);
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
