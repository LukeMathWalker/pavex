use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::middleware::Next;
use pavex::response::Response;

pub struct A;

pub fn a() -> A {
    todo!()
}
pub fn handler(_a: A) -> Response {
    todo!()
}

pub fn mw<C>(_next: Next<C>, _a: A) -> Response
where
    C: std::future::IntoFuture<Output = Response>,
{
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(crate::a));
    bp.wrap(f!(crate::mw));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
