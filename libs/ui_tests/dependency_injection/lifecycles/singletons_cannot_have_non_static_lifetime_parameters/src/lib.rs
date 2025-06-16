use pavex::blueprint::{from, Blueprint};
use pavex::http::StatusCode;

pub struct A;

#[derive(Clone)]
pub struct B<'a>(pub &'a A);

#[pavex::methods]
impl<'a> B<'a> {
    #[singleton(clone_if_necessary)]
    pub fn new(a: &'a A) -> Self {
        B(a)
    }
}

#[pavex::singleton(id = "A_")]
pub fn a() -> A {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_b: B<'_>) -> StatusCode {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
