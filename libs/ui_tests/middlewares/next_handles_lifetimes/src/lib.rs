use pavex::middleware::Next;
use pavex::response::Response;
use pavex::{blueprint::from, Blueprint};

pub struct A;

pub struct C;

pub struct B<'a>(pub &'a A);

#[pavex::request_scoped(id = "A_")]
pub fn a() -> A {
    todo!()
}

#[pavex::request_scoped(id = "C_")]
pub fn c() -> C {
    todo!()
}

#[pavex::request_scoped(id = "B_")]
pub fn b<'a>(_a: &'a A, _c: &'a C) -> B<'a> {
    todo!()
}

#[pavex::wrap]
pub fn mw<T>(_next: Next<T>, _b: B<'_>) -> Response
where
    T: IntoFuture<Output = Response>,
{
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler(_a: &A, _c: &C) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate, pavex]);
    bp.wrap(MW);
    bp.routes(from![crate]);
    bp
}
