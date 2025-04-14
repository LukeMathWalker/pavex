use pavex::blueprint::{from, router::GET, Blueprint};
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

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[pavex::config(key = "a1")]
pub struct A1;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
// Same key, different type. The only type of conflict
// you can have with annotation-only config types.
#[pavex::config(key = "a1")]
pub struct B1;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
// Key conflict *and* type conflict with a blueprint-provided
// config type.
#[pavex::config(key = "c")]
pub struct C1;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();

    bp.import(from![crate]);

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
