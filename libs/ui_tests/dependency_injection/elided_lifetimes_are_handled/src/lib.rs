use pavex::blueprint::{from, Blueprint};
use pavex::f;
use pavex::http::StatusCode;

pub struct A;

impl Default for A {
    fn default() -> Self {
        Self::new()
    }
}

impl A {
    pub fn new() -> A {
        todo!()
    }
}

pub struct Generic<'a>(pub &'a A);

impl<'a> Generic<'a> {
    pub fn new(_config: &'a A) -> Generic<'a> {
        todo!()
    }
}

#[pavex::get(path = "/")]
pub fn handler(_generic: Generic<'_>) -> StatusCode {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.singleton(f!(self::A::new));
    bp.transient(f!(self::Generic::new));
    bp.routes(from![crate]);
    bp
}
