use pavex::http::StatusCode;
use pavex_builder::{constructor::Lifecycle, f, router::GET, Blueprint};

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
    bp.constructor(f!(crate::a), Lifecycle::Transient);
    bp.constructor(f!(crate::b), Lifecycle::Transient);
    bp.constructor(f!(crate::c), Lifecycle::Transient);
    bp.route(GET, "/", f!(crate::handler));
    bp
}
