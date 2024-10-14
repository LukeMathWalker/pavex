use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::request::RequestHead;
use pavex::response::Response;

// The call graph looks like this:
//
// Request
//  / \
// B   |&
//  \  |
// handler
//
// `Request` cannot be borrowed by `handler` after it has been moved to construct `B`.
// Pavex should detect this and report an error since `Request` is a framework built-in type and
// it is not marked as `CloneIfNecessary`.

pub struct B;

pub fn b(_r: RequestHead) -> B {
    todo!()
}

pub fn handler(_r: &RequestHead, _b: B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(crate::b));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
