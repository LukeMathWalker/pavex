use pavex::blueprint::{from, Blueprint};
use pavex::response::Response;
use pavex::{get, methods};
use std::borrow::Cow;

pub trait LocalTrait {
    fn new() -> Self;
}

#[methods]
// Local trait for foreign type, with a generic (elided) lifetime parameter.
impl LocalTrait for &str {
    #[transient]
    fn new() -> Self {
        todo!()
    }
}

#[methods]
// Local trait for foreign type, with a generic (inferred) lifetime parameter.
impl LocalTrait for Cow<'_, str> {
    #[transient]
    fn new() -> Self {
        todo!()
    }
}

#[methods]
// Local trait for foreign type, with a generic (explicit) lifetime parameter.
impl<'a> LocalTrait for Cow<'a, [u8]> {
    #[transient]
    fn new() -> Self {
        todo!()
    }
}

pub trait LocalTraitWithLifetime<'a> {
    fn new(source: &'a str) -> &'a Self;
}

#[methods]
// Local trait for foreign type, with a generic (inferred) lifetime parameter.
impl LocalTraitWithLifetime<'_> for [u8] {
    #[transient]
    fn new(_source: &str) -> &Self {
        todo!()
    }
}

#[methods]
// Local trait for foreign type, with a generic (explicit) lifetime parameter.
impl<'a> LocalTraitWithLifetime<'a> for [u32] {
    #[transient]
    fn new(_source: &'a str) -> &'a Self {
        todo!()
    }
}

pub trait LocalGenericTrait<T> {
    fn t() -> T;
}

pub struct A;

// Local trait implemented for a local type.
// The trait is generic, but its only generic parameter is assigned
// a concrete type.
#[methods]
impl LocalGenericTrait<A> for A {
    #[request_scoped]
    fn t() -> A {
        A
    }
}

pub struct B;

// Foreign trait implemented for a local type.
#[methods]
impl Default for B {
    #[request_scoped]
    fn default() -> B {
        B
    }
}

pub struct C<T>(T);

// Foreign trait implemented for a local type.
// The local type is generic, but its generic parameter
// is assigned a concrete type.
#[methods]
impl Default for C<A> {
    #[request_scoped]
    // Using `Self` rather than the actual type.
    fn default() -> Self {
        todo!()
    }
}

pub struct D<T>(T);

#[methods]
/// Local trait implemented for a local type.
/// Both are generic, but all generic parameters are assigned
/// a concrete type.
impl LocalGenericTrait<D<A>> for D<A> {
    #[request_scoped]
    fn t() -> Self {
        todo!()
    }
}

#[get(path = "/")]
pub fn handler(
    _a: &A,
    _b: &B,
    _c: &C<A>,
    _d: &D<A>,
    _str: &str,
    _cow_str: Cow<'_, str>,
    _cow_bytes: Cow<'_, [u8]>,
    _byte_slice: &[u8],
    _u32_slice: &[u32],
) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
