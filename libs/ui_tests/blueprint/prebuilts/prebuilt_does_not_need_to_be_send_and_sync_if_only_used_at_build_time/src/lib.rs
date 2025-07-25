use pavex::Response;
use pavex::{blueprint::from, Blueprint};
use std::rc::Rc;

#[pavex::prebuilt(id = "A_")]
#[derive(Clone)]
pub struct A(pub Rc<String>);

#[derive(Clone)]
pub struct B;

#[pavex::singleton(id = "B_")]
pub fn b(_a: A) -> B {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_b: &B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
