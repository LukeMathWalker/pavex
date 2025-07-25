use pavex::Response;
use pavex::{blueprint::from, Blueprint};

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

#[pavex::request_scoped(id = "A_")]
pub fn a() -> Result<A, Error> {
    todo!()
}

#[pavex::error_handler]
pub fn error_handler(#[px(error_ref)] _e: &Error, _b: B) -> Response {
    todo!()
}

#[pavex::request_scoped(id = "B_")]
pub fn b() -> B {
    todo!()
}

#[pavex::request_scoped(id = "C_")]
pub fn c(_a: A, _b: B) -> C {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler(_b: B, _c: C) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
