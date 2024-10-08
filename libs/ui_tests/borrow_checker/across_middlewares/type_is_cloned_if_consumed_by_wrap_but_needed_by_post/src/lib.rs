use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::middleware::Next;
use pavex::response::Response;
use std::future::IntoFuture;

#[derive(Clone)]
pub struct A;

pub fn a() -> A {
    todo!()
}

pub fn wrap<T>(_next: Next<T>, _a: A) -> Response
where
    T: IntoFuture<Output = Response>,
{
    todo!()
}

pub fn post(_response: Response, _a: &A) -> Response {
    todo!()
}

pub fn handler() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(crate::a)).clone_if_necessary();
    bp.post_process(f!(crate::post));
    bp.wrap(f!(crate::wrap));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
