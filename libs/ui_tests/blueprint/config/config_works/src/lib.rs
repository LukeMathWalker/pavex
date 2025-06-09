use pavex::blueprint::{from, router::GET, Blueprint};
use pavex::response::Response;
use pavex::{f, t};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct A;

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct B<T>(pub T);

#[derive(Debug, Clone, serde::Deserialize)]
#[pavex::config(key = "a1")]
pub struct A1;

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[pavex::config(key = "b1", default_if_missing)]
pub struct B1(pub String);

#[pavex::config(key = "c")]
// Re-exported type.
pub use sub::C;

#[pavex::config(key = "e")]
// Re-exported type with rename.
pub use sub::D as E;

#[pavex::config(key = "f")]
// Re-exported type from another crate.
pub use dep::F;

#[pavex::config(key = "g")]
// Re-exported type from another crate, with rename.
pub use dep::Z as G;

mod sub {
    #[derive(Debug, Clone, serde::Deserialize)]
    pub struct C;
    #[derive(Debug, Clone, serde::Deserialize)]
    pub struct D;
}

pub fn handler(
    _a: &A,
    _b: &B<String>,
    _a1: &A1,
    _b1: &B1,
    _c: &C,
    _d: Vec<String>,
    _e: &E,
    _f: &F,
    _g: &G,
) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.config("a", t!(crate::A));
    bp.config("b", t!(crate::B<std::string::String>))
        .default_if_missing();
    bp.config("d", t!(std::vec::Vec<std::string::String>));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
