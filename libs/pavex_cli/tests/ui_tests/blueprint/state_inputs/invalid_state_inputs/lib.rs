use pavex::blueprint::{router::GET, Blueprint};
use pavex::response::Response;
use pavex::{f, t};

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
    bp.prebuilt(t!(crate::A));
    bp.prebuilt(t!(crate::B));
    bp.prebuilt(t!(crate::C));
    bp.prebuilt(t!(crate::D));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
