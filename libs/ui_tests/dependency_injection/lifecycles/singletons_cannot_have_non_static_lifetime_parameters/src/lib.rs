use pavex::blueprint::{from, Blueprint};
use pavex::f;
use pavex::http::StatusCode;

pub struct A;

#[derive(Clone)]
pub struct B<'a>(pub &'a A);

impl<'a> B<'a> {
    pub fn new(a: &'a A) -> Self {
        B(a)
    }
}

pub fn a() -> A {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_b: B<'_>) -> StatusCode {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.singleton(f!(self::a));
    bp.singleton(f!(self::B::new)).clone_if_necessary();
    bp.routes(from![crate]);
    bp
}
