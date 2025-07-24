use pavex::middleware::Next;
use pavex::Response;
use pavex::{blueprint::from, Blueprint};

pub struct A;

#[pavex::request_scoped(id = "A_")]
pub fn a() -> A {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler(_a: A) -> Response {
    todo!()
}

#[pavex::wrap]
pub fn mw<C>(_next: Next<C>, _a: A) -> Response
where
    C: std::future::IntoFuture<Output = Response>,
{
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);

    bp.wrap(MW);
    bp.routes(from![crate]);
    bp
}
