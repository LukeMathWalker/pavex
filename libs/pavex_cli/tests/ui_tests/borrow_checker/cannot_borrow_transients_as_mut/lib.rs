use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

#[derive(Clone)]
pub struct A;

pub fn build() -> A {
    A
}

pub fn handler(_a: &mut A) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.transient(f!(self::build));
    bp.route(GET, "/", f!(self::handler));
    bp
}
