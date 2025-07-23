use pavex::{blueprint::from, Blueprint};
use pavex::middleware::Next;
use pavex::response::Response;

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

#[pavex::wrap]
pub fn wrapper<F: IntoFuture<Output = Response>>(_next: Next<F>, _c: C) -> Response {
    todo!()
}

#[pavex::request_scoped(id = "A_")]
pub fn a() -> A {
    todo!()
}

#[pavex::request_scoped(id = "C_")]
pub fn c(_a: &A) -> C {
    todo!()
}

#[pavex::request_scoped(id = "B_")]
pub fn b(_a: &A) -> B {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_b: B, _a: &mut A) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.wrap(WRAPPER);
    bp.routes(from![crate]);
    bp
}
