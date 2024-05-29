use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

#[derive(Clone)]
pub struct A;

#[derive(Clone)]
pub struct B<T>(T);

#[derive(Clone)]
pub struct C<'a>(&'a str);

pub fn handler(a: &A, b: &B<String>, c: C<'static>) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.state_input(f!(crate::A));
    bp.state_input(f!(crate::B<std::string::String>));
    bp.state_input(f!(crate::C<'static>));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
