use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::middleware::Next;
use pavex::response::Response;
use std::future::IntoFuture;

// The call graph for the handler looks like this:
//
//  &mut A
//  /    \
// |      |
// |      B
// |      |
// |      |
// handler
//
// Pavex should correctly determine that this is not an issue,
// because `B` only borrows `A` immutably.
// Furthermore, Pavex should correctly pass `&mut A` to `B`'s constructor,
// relying on the compiler's deref coercion to do its magic.

pub struct A;

pub struct B;

pub struct C;

pub fn wrapper<F: IntoFuture<Output = Response>>(_next: Next<F>, _c: C) -> Response {
    todo!()
}

pub fn a() -> A {
    todo!()
}

pub fn c(_a: &A) -> C {
    todo!()
}

pub fn b(_a: &A) -> B {
    todo!()
}

pub fn handler(_b: B, _a: &mut A) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(crate::a));
    bp.request_scoped(f!(crate::b));
    bp.request_scoped(f!(crate::c));
    bp.wrap(f!(crate::wrapper));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
