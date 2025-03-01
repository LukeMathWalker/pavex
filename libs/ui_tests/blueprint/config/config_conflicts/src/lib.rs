use pavex::blueprint::{router::GET, Blueprint};
use pavex::response::Response;
use pavex::{f, t};

pub fn handler(_a: &A, _b: &B, _c: &C) -> Response {
    todo!()
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct A;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct B;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct C;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    // Same key, different types.
    bp.config("a", t!(self::C));
    bp.config("a", t!(self::B));

    // Different key, same type.
    bp.config("b", t!(self::A));
    bp.config("c", t!(self::A));

    // Key conflict *and* type conflict
    bp.config("c", t!(self::B));

    bp.route(GET, "/", f!(crate::handler));
    bp
}
