use pavex_builder::{constructor::Lifecycle, f, router::GET, Blueprint};
use pavex_runtime::http::StatusCode;

#[derive(Clone)]
pub struct A;

pub struct B;

pub struct C;

pub fn a(_b: B, _c: C) -> A {
    todo!()
}

pub fn b() -> B {
    todo!()
}

pub fn c() -> C {
    todo!()
}

pub fn handler(_a: &A) -> StatusCode {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::a), Lifecycle::Singleton);
    bp.constructor(f!(crate::b), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::c), Lifecycle::Transient);
    bp.route(GET, "/", f!(crate::handler));
    bp
}
