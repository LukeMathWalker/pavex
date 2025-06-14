use pavex::blueprint::{from, Blueprint};
use pavex::response::Response;

#[pavex::get(path = "/")]
pub fn handler(_a: &A, _b: &B, _c: &C, _d: &D) -> Response {
    todo!()
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[pavex::config(key = "a", id = "CONFIG_A")]
pub struct A;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
// Same key as A, but different type.
#[pavex::config(key = "a", id = "CONFIG_B")]
pub struct B;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[pavex::config(key = "c", id = "CONFIG_C")]
pub struct C;

#[pavex::config(key = "d", id = "CONFIG_D")]
// Different key, same type as C.
pub use C as D;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
