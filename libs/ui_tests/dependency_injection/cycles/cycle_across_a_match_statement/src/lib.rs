use pavex::blueprint::{from, Blueprint};
use pavex::http::StatusCode;

// The dependency graph for the request handler looks like this:
//
//          ┌─────────────┐
//    ┌────▶│ Result<A, E>│─────┐
//    │     └─────────────┘     │
//    │                         │
//    │                         ▼
// ┌─────┐                   ┌─────┐
// │  B  │◀──────────────────│Ok(A)│
// └─────┘                   └─────┘
//    │
//    └─────────┐
//              ▼
//       ┌────────────┐
//       │  Request   │
//       │  handler   │
//       └────────────┘
//
// There is a cyclic dependency, and it's not allowed.

pub struct A;
pub struct Error;
pub struct B;

#[pavex::request_scoped(id = "A_")]
pub fn a(_b: &B) -> Result<A, Error> {
    todo!()
}

#[pavex::transient(id = "B_")]
pub fn b(_c: &A) -> B {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_b: &B) -> StatusCode {
    todo!()
}

#[pavex::error_handler]
pub fn error_handler(_e: &Error) -> StatusCode {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
