use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
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

pub fn a(_b: &B) -> Result<A, Error> {
    todo!()
}

pub fn b(_c: &A) -> B {
    todo!()
}

pub fn handler(_b: &B) -> StatusCode {
    todo!()
}

pub fn error_handler(_e: &Error) -> StatusCode {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::a), Lifecycle::RequestScoped)
        .error_handler(f!(crate::error_handler));
    bp.constructor(f!(crate::b), Lifecycle::Transient);
    bp.route(GET, "/", f!(crate::handler));
    bp
}
