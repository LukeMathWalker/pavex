use std::future::IntoFuture;

use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::middleware::Next;
use pavex::response::Response;

pub struct A;

pub struct C;

pub struct B<'a>(pub &'a A);

pub fn a() -> A {
    todo!()
}

pub fn c() -> C {
    todo!()
}

pub fn b<'a>(_a: &'a A, _c: &'a C) -> B<'a> {
    todo!()
}

pub fn mw<T>(_next: Next<T>, _b: B<'_>) -> Response
where
    T: IntoFuture<Output = Response>,
{
    todo!()
}

pub fn handler(_a: &A, _c: &C) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::a), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::b), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::c), Lifecycle::RequestScoped);
    bp.wrap(f!(crate::mw));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
