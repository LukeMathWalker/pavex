use pavex::blueprint::{from, Blueprint};
use pavex::response::Response;

#[derive(Clone)]
#[pavex::prebuilt(id = "B_")]
pub struct B<T>(T);

#[derive(Clone)]
#[pavex::prebuilt(id = "D_")]
pub struct D<T, S, Z>(T, S, Z);

#[derive(Clone)]
#[pavex::prebuilt(id = "A_")]
pub struct A<'a> {
    pub a: &'a str,
}

#[derive(Clone)]
#[pavex::prebuilt(id = "C_")]
pub struct C<'a, 'b, 'c> {
    pub a: &'a str,
    pub b: &'b str,
    pub c: &'c str,
}

#[pavex::get(path = "/")]
pub fn handler(_a: A, _b: B<String>, _c: C, _d: D<String, u16, u64>) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
