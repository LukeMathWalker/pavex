use pavex::http::StatusCode;
use pavex::{blueprint::from, Blueprint};

// The dependency graph for the request handler looks like this:
//
//          ┌─────┐
//    ┌────▶│  A  │─────┐
//    │     └─────┘     │
//    │                 │
//    │                 ▼
// ┌─────┐           ┌─────┐
// │  B  │◀──────────│  C  │
// └─────┘           └─────┘
//    │
//    └─────────┐
//              ▼
//       ┌────────────┐
//       │  Request   │
//       │  handler   │
//       └────────────┘
//
// The request needs `B`, which needs `C`, which needs `A`, which needs `B`.
// This is a cyclic dependency, and it's not allowed.

pub struct A;
pub struct B;
pub struct C;

#[pavex::transient(id = "A_")]
pub fn a(_b: &B) -> A {
    todo!()
}

#[pavex::transient(id = "B_")]
pub fn b(_c: &C) -> B {
    todo!()
}

#[pavex::transient(id = "C_")]
pub fn c(_a: &A) -> C {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_b: &B) -> StatusCode {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
