use pavex::blueprint::{from, router::GET, Blueprint};
use pavex::response::Response;
use pavex::{f, t};

#[derive(Clone)]
pub struct A;

#[derive(Clone)]
pub struct B;

#[derive(Clone)]
pub struct C;

#[derive(Clone)]
#[pavex::config(key = "2numbersfirst")]
pub struct A1;

#[derive(Clone)]
#[pavex::config(key = "")]
pub struct B1;

#[derive(Clone)]
#[pavex::config(key = "with-a-dash")]
pub struct C1;

pub fn handler(_a: A, _b: B, _c: C) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.config("12a", t!(crate::A));
    bp.config("", t!(crate::B));
    bp.config("my-key", t!(crate::C));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
