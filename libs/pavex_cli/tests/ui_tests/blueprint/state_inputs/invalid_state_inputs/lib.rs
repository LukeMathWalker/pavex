use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

#[derive(Clone)]
pub struct B<T>(T);

#[derive(Clone)]
pub struct D<T, S, Z>(T, S, Z);

#[derive(Clone)]
pub struct A<'a> {
    a: &'a str,
}

#[derive(Clone)]
pub struct C<'a, 'b, 'c> {
    a: &'a str,
    b: &'b str,
    c: &'c str,
}

pub fn handler(a: A, b: B<String>, c: C, d: D<String, u16, u64>) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.state_input(f!(crate::A));
    bp.state_input(f!(crate::B));
    bp.state_input(f!(crate::C));
    bp.state_input(f!(crate::D));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
