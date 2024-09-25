use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::middleware::{Next, Processing};
use pavex::response::Response;
use std::future::IntoFuture;

pub struct A;

pub fn a() -> A {
    todo!()
}

pub fn pre(_a: &A) -> Processing {
    todo!()
}

pub fn wrap<T: IntoFuture<Output = Response>>(next: Next<T>, _a: &A) -> Response {
    todo!()
}

pub fn post(_response: Response, _a: A) -> Response {
    todo!()
}

pub fn handler() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(crate::a));
    bp.pre_process(f!(crate::pre));
    bp.post_process(f!(crate::post));
    bp.wrap(f!(crate::wrap));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
