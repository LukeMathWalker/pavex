use pavex::blueprint::{from, router::GET, Blueprint};
use pavex::response::Response;
use pavex::{f, t};

#[derive(Debug, Clone, serde::Deserialize)]
// Won't be included in the generated `ApplicationConfig`.
pub struct A;

#[derive(Debug, Clone, serde::Deserialize)]
#[pavex::config(key = "a1")]
// Won't be included in the generated `ApplicationConfig`.
pub struct A1;

#[derive(Debug, Clone, serde::Deserialize)]
// Will be included in the generated `ApplicationConfig`.
pub struct B;

#[derive(Debug, Clone, serde::Deserialize)]
// Will be included in the generated `ApplicationConfig`.
#[pavex::config(key = "b1", include_if_unused)]
pub struct B1;

pub fn handler() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.config("a", t!(crate::A));
    bp.config("b", t!(crate::B)).include_if_unused();
    bp.route(GET, "/", f!(crate::handler));
    bp
}
