use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::http::StatusCode;

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

pub fn a(_b: &B) -> A {
    todo!()
}

pub fn b(_c: &C) -> B {
    todo!()
}

pub fn c(_a: &A) -> C {
    todo!()
}

pub fn handler(_b: &B) -> StatusCode {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::a), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::b), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::c), Lifecycle::RequestScoped);
    bp.route(GET, "/", f!(crate::handler));
    bp
}
