use pavex::blueprint::{from, Blueprint};
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

#[pavex::get(path = "/home")]
pub fn handler(_a: A, _b: B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
