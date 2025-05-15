use pavex::blueprint::{from, router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

#[derive(Clone)]
#[pavex::config(key = "a")]
pub struct A<'a> {
    pub a: &'a str,
}

#[derive(Clone)]
/// One generic parameter
#[pavex::config(key = "b")]
pub struct B<T>(T);

#[derive(Clone)]
/// More than one lifetime
#[pavex::config(key = "c")]
pub struct C<'a, 'b, 'c> {
    pub a: &'a str,
    pub b: &'b str,
    pub c: &'c str,
}

#[derive(Clone)]
/// More than one generic parameter
#[pavex::config(key = "d")]
pub struct D<T, S, Z>(T, S, Z);

#[derive(Clone)]
#[allow(dead_code)]
#[pavex::config(key = "f")]
// Some static, some elided.
pub struct F<'a, 'b>(std::borrow::Cow<'a, str>, &'b str);

pub fn handler() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.route(GET, "/", f!(crate::handler));
    bp
}
