use pavex::blueprint::{from, router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

pub struct A;

#[pavex::request_scoped]
pub fn build() -> A {
    A
}

pub fn handler(_a: &mut A) -> Response {
    todo!()
}

#[pavex::get(path = "/annotation")]
pub fn handler_ann(_a: &mut A) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp.route(GET, "/", f!(self::handler));
    bp
}
