use pavex::middleware::Next;
use pavex::Response;
use pavex::{blueprint::from, Blueprint};

// The call graph looks like this:
//
//       A
//    /     \
//   /        \
//  /          \
// middleware   |&
//  \          /
//   \       /
//    handler
//
// `A` cannot be borrowed by `handler` after it has been moved to invoke `middleware`.
// `A` is cloneable though!
// Pavex should detect this and clone `A` before calling the middleware.

#[derive(Clone)]
pub struct A;

#[pavex::request_scoped(clone_if_necessary, id = "A_")]
pub fn a() -> A {
    todo!()
}

#[pavex::wrap]
pub fn mw<T: std::future::IntoFuture<Output = Response>>(_a: A, _next: Next<T>) -> Response {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler(_a: &A) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.wrap(MW);
    bp.routes(from![crate]);
    bp
}
