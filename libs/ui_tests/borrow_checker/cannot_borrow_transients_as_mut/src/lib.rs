use pavex::{blueprint::from, Blueprint};
use pavex::response::Response;

#[derive(Clone)]
pub struct A;

#[pavex::transient]
pub fn build() -> A {
    A
}

#[pavex::get(path = "/")]
pub fn handler(_a: &mut A) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
