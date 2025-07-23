use pavex::response::Response;
use pavex::{blueprint::from, Blueprint};

#[derive(Debug, Clone, serde::Deserialize)]
#[pavex::config(key = "a", id = "CONFIG_A")]
// Won't be included in the generated `ApplicationConfig`.
pub struct A;

#[derive(Debug, Clone, serde::Deserialize)]
// Will be included in the generated `ApplicationConfig`.
#[pavex::config(key = "b", id = "CONFIG_B", include_if_unused)]
pub struct B;

#[pavex::get(path = "/")]
pub fn handler() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
