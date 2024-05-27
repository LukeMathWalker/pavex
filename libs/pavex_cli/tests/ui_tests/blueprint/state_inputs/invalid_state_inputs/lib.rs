use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

#[derive(Clone)]
pub struct B<T>(T);

#[derive(Clone)]
pub struct A<'a> {
    a: &'a str,
}

pub fn handler(a: A, b: B<String>) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.state_input(f!(crate::A));
    bp.state_input(f!(crate::B));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
