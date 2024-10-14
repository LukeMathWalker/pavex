use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

// The call graph looks like this:
//
//  A   B
// &| X |&
//  D   C
//   \ /
// handler
//
// If `D` is constructed before `C`, then `A` cannot be borrowed by `C`'s constructor after it
// has been moved to construct `D`.
// If `C` is constructed before `D`, then `B` cannot be borrowed by `D`'s constructor after it
// has been moved to construct `C`.
//
// Both `A` and `B` are cloneable though!
// Pavex should detect this and insert a `Clone` to break the empasse.

#[derive(Clone)]
pub struct A;

#[derive(Clone)]
pub struct B;

pub struct C;

pub struct D;

pub fn a() -> A {
    todo!()
}

pub fn b() -> B {
    todo!()
}

pub fn c(_a: A, _b: &B) -> C {
    todo!()
}

pub fn d(_a: &A, _b: B) -> D {
    todo!()
}

pub fn handler(_c: C, _d: D) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(crate::a)).clone_if_necessary();
    bp.request_scoped(f!(crate::b)).clone_if_necessary();
    bp.request_scoped(f!(crate::c));
    bp.request_scoped(f!(crate::d));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
