use pavex::request::RequestHead;
use pavex::Response;
use pavex::{blueprint::from, Blueprint};

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

#[pavex::request_scoped(id = "B_")]
pub fn b(_r: RequestHead) -> B {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler(_r: &RequestHead, _b: B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
