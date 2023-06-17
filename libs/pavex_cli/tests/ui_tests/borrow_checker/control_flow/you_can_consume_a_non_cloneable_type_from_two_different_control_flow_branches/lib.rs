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
//  |    /  \  |
// handler   error h
//
// The type B is not cloneable and has two consumers that take it by value (`handler` and `error handler`).
// But those consumers are in different control flow branches, so they'll never be invoked
// one after the other, therefore the graph is fine as is and Pavex should accept it.

pub struct A;

#[derive(Debug)]
pub struct Error;

pub struct B;

pub fn a() -> Result<A, Error> {
    todo!()
}

pub fn error_handler(_e: &Error, _b: B) -> Response {
    todo!()
}

pub fn b() -> B {
    todo!()
}

pub fn handler(_a: A, _b: B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::a), Lifecycle::RequestScoped)
        .error_handler(f!(crate::error_handler));
    bp.constructor(f!(crate::b), Lifecycle::RequestScoped);
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
