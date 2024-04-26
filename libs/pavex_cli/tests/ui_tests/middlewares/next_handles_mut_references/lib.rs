use std::future::IntoFuture;

use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::middleware::Next;
use pavex::response::Response;

pub struct A;

pub fn a() -> A {
    A
}

pub fn mw<T>(_next: Next<T>) -> Response
where
    T: IntoFuture<Output = Response>,
{
    todo!()
}

pub fn post(_a: A, _r: Response) -> Response {
    todo!()
}

pub fn handler(_a: &mut A) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(crate::a));
    bp.post_process(f!(crate::post));
    bp.wrap(f!(crate::mw));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
