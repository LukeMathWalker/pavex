use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::middleware::Next;
use pavex::response::Response;

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

pub fn a() -> A {
    todo!()
}

pub fn mw<T: std::future::IntoFuture<Output = Response>>(_a: A, next: Next<T>) -> Response {
    todo!()
}

pub fn handler(_a: &A) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(crate::a)).clone_if_necessary();
    bp.wrap(f!(crate::mw));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
