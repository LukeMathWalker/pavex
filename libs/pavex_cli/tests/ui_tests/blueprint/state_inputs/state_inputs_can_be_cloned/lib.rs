use pavex::blueprint::{constructor::CloningStrategy, router::GET, Blueprint};
use pavex::response::Response;
use pavex::{f, t};

#[derive(Clone)]
pub struct A;

#[derive(Clone)]
pub struct B;

pub fn b(a: A) -> B {
    todo!()
}

pub fn handler(a: A, b: &B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.state_input(t!(crate::A))
        .cloning(CloningStrategy::CloneIfNecessary);
    bp.singleton(f!(crate::b));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
