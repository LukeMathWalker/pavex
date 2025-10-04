use std::future::IntoFuture;

use pavex::middleware::Next;
use pavex::Response;
use pavex::{blueprint::from, Blueprint};

pub struct A;

#[pavex::request_scoped(id = "A_")]
pub fn a() -> A {
    A
}

#[pavex::wrap]
pub fn mw<T>(_next: Next<T>) -> Response
where
    T: IntoFuture<Output = Response>,
{
    todo!()
}

#[pavex::post_process]
pub fn post(_a: A, _r: Response) -> Response {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler(_a: &mut A) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate, pavex]);
    bp.post_process(POST);
    bp.wrap(MW);
    bp.routes(from![crate]);
    bp
}
