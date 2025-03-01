use pavex::blueprint::{router::GET, Blueprint};
use pavex::response::Response;
use pavex::{f, t};

#[derive(Clone)]
pub struct A;

#[derive(Clone)]
pub struct B;

#[derive(Clone)]
pub struct C;

pub fn handler(_a: A, _b: B, _c: C) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.config("12a", t!(crate::A));
    bp.config("", t!(crate::B));
    bp.config("my-key", t!(crate::C));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
