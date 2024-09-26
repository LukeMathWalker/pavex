use pavex::blueprint::{constructor::CloningStrategy, router::GET, Blueprint};
use pavex::response::Response;
use pavex::{f, t};

pub struct A;
pub struct B;
pub struct C;

pub fn singleton() -> A {
    todo!()
}

pub fn request_scoped() -> B {
    todo!()
}

pub fn handler() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.prebuilt(t!(crate::C)).clone_if_necessary();
    bp.singleton(f!(crate::singleton)).clone_if_necessary();
    bp.request_scoped(f!(crate::request_scoped))
        .clone_if_necessary();
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
