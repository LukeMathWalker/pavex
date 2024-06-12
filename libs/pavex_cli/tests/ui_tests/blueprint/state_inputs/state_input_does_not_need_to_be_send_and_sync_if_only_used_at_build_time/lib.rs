use pavex::blueprint::{router::GET, Blueprint};
use pavex::response::Response;
use pavex::{f, t};
use std::rc::Rc;

pub struct A(Rc<String>);

#[derive(Clone)]
pub struct B;

pub fn b(a: A) -> B {
    todo!()
}

pub fn handler(b: &B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.prebuilt(t!(crate::A));
    bp.singleton(f!(crate::b));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
