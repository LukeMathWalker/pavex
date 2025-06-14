use pavex::blueprint::{from, Blueprint};
use pavex::f;
use pavex::response::Response;

#[derive(Clone)]
pub struct A;

pub fn build() -> A {
    A
}

#[pavex::get(path = "/")]
pub fn handler(_a: &mut A) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.transient(f!(self::build));
    bp.routes(from![crate]);
    bp
}
