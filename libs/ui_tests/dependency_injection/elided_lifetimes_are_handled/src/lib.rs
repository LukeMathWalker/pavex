use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::http::StatusCode;

pub struct A;

impl A {
    pub fn new() -> A {
        todo!()
    }
}

pub struct Generic<'a>(&'a A);

impl<'a> Generic<'a> {
    pub fn new(config: &'a A) -> Generic<'a> {
        todo!()
    }
}

pub fn handler<T>(generic: T) -> StatusCode {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.singleton(f!(self::A::new));
    bp.transient(f!(self::Generic::new));
    bp.route(GET, "/", f!(self::handler::<self::Generic>));
    bp
}
