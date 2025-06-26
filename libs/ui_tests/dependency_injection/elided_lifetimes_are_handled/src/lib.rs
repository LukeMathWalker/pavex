use pavex::http::StatusCode;
use pavex::{blueprint::from, Blueprint};

pub struct A;

impl Default for A {
    fn default() -> Self {
        Self::new()
    }
}

#[pavex::methods]
impl A {
    #[singleton]
    pub fn new() -> A {
        todo!()
    }
}

pub struct Generic<'a>(pub &'a A);

#[pavex::methods]
impl<'a> Generic<'a> {
    #[transient]
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
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
