use pavex::blueprint::{router::GET, Blueprint};
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

pub fn handler(_b: B<'_>) -> StatusCode {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.singleton(f!(self::a));
    bp.singleton(f!(self::B::new)).clone_if_necessary();
    bp.route(GET, "/", f!(self::handler));
    bp
}
