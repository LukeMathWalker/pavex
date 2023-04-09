use std::path::PathBuf;

use pavex_builder::{constructor::Lifecycle, f, router::GET, Blueprint};
use pavex_runtime::response::Response;

pub struct A;

pub struct B;

pub fn a() -> A {
    todo!()
}

pub fn b(_a: A) -> B {
    todo!()
}

pub fn handler(_a: &A, _b: B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::a), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::b), Lifecycle::RequestScoped);
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
