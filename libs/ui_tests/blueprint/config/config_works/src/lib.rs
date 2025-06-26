use pavex::response::Response;
use pavex::{blueprint::from, Blueprint};

#[derive(Debug, Clone, serde::Deserialize)]
#[pavex::config(key = "a", id = "CONFIG_A")]
pub struct A;

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[pavex::config(key = "b", id = "CONFIG_B", default_if_missing)]
pub struct B(pub String);

#[pavex::config(key = "c", id = "CONFIG_C")]
// Re-exported type.
pub use sub::C;

#[pavex::config(key = "e", id = "CONFIG_E")]
// Re-exported type with rename.
pub use sub::D as E;

#[pavex::config(key = "f", id = "CONFIG_F")]
// Re-exported type from another crate.
pub use dep::F;

#[pavex::config(key = "g", id = "CONFIG_G")]
// Re-exported type from another crate, with rename.
pub use dep::Z as G;

mod sub {
    #[derive(Debug, Clone, serde::Deserialize)]
    pub struct C;
    #[derive(Debug, Clone, serde::Deserialize)]
    pub struct D;
}

#[pavex::get(path = "/")]
pub fn handler(_a: &A, _b: &B, _c: &C, _e: &E, _f: &F, _g: &G) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
