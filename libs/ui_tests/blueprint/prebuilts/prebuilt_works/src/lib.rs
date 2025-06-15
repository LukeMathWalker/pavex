use pavex::blueprint::{from, Blueprint};
use pavex::response::Response;

#[derive(Clone)]
#[pavex::prebuilt(clone_if_necessary, id = "A_")]
pub struct A;

#[pavex::prebuilt(id = "C_")]
// Re-exported type.
pub use sub::C;

#[pavex::prebuilt(id = "E_")]
// Re-exported type with rename.
pub use sub::D as E;

#[pavex::prebuilt(id = "F_")]
// Re-exported type from another crate.
pub use dep::F;

#[pavex::prebuilt(id = "G_")]
// Re-exported type from another crate, with rename.
pub use dep::Z as G;

mod sub {
    #[derive(Clone)]
    pub struct C;
    #[derive(Clone)]
    pub struct D;
}

#[pavex::get(path = "/")]
pub fn handler(_a: &A, _c: &C, _e: &E, _f: &F, _g: &G) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
