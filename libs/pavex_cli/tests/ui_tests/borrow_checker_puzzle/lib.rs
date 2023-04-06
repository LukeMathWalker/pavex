use std::path::PathBuf;

use pavex_builder::{constructor::Lifecycle, f, router::GET, Blueprint};
use pavex_runtime::response::Response;

pub struct A;

pub struct B;

pub struct C;

pub struct D;

pub fn a() -> A {
    todo!()
}

pub fn b() -> B {
    todo!()
}

pub fn c(_a: A, _b: &B) -> C {
    todo!()
}

pub fn d(_a: &A, _b: B) -> D {
    todo!()
}

pub fn handler(_c: C, _d: D) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::a), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::b), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::c), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::d), Lifecycle::RequestScoped);
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
