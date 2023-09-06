use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::middleware::Next;
use pavex::response::Response;
use std::future::IntoFuture;

pub struct A;

pub struct B<'a>(&'a A);

pub fn a() -> A {
    todo!()
}

pub fn b(_a: &A) -> B<'_> {
    todo!()
}

pub fn mw<'l, T>(_next: Next<T>, _b: B<'l>) -> Response
where
    T: IntoFuture<Output = Response>,
{
    todo!()
}

pub fn handler(_a: &A) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::a), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::b), Lifecycle::RequestScoped);
    bp.wrap(f!(crate::mw));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
