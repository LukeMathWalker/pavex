use pavex::{f, blueprint::{from, Blueprint}};
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
    bp.singleton(f!(self::build));
    bp.routes(from![crate]);
    bp
}
