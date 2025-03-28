use pavex::blueprint::{from, router::GET, Blueprint};
use pavex::f;

#[derive(Clone)]
pub struct A;

#[pavex::singleton(clone_if_necessary)]
/// As simple as it gets.
pub fn a() -> A {
    A
}

pub struct B<T>(T);

#[pavex::request_scoped]
/// Generic, but all generic parameters are used in the output type.
pub fn b<T>(_i: T) -> B<T> {
    todo!()
}

pub struct C;

#[pavex::transient(error_handler = "crate::error_handler")]
/// Fallible.
pub fn c(_b: &B<A>) -> Result<C, pavex::Error> {
    todo!()
}

pub struct D<'a> {
    _c: &'a C,
    _a: &'a A,
}

#[pavex::constructor(transient)]
/// With a lifetime parameter.
pub fn d<'a>(_c: &'a C, _a: &'a A) -> D<'a> {
    todo!()
}

pub fn error_handler(_error: &pavex::Error) -> pavex::response::Response {
    todo!()
}

pub struct E;

impl E {
    // Simple method constructor.
    #[pavex::request_scoped]
    pub fn new() -> Self {
        Self
    }
}

pub struct F<'a> {
    _e: &'a E,
}

impl F<'_> {
    // With an (elided) lifetime parameter.
    #[pavex::request_scoped]
    pub fn new(_e: &E) -> Self {
        todo!()
    }
}

pub struct G<T>(T);

impl<T> G<T> {
    // With a generic parameter in the output type.
    #[pavex::request_scoped]
    pub fn new(_t: T) -> Self {
        todo!()
    }
}

pub struct H<T>(T);

impl H<A> {
    // With a generic parameter that's specified in the `impl` block definition.
    #[pavex::request_scoped]
    pub fn with_a() -> Self {
        todo!()
    }
}

impl H<E> {
    // With a generic parameter that's specified in the `impl` block definition.
    #[pavex::request_scoped]
    pub fn with_e() -> Self {
        todo!()
    }
}

pub fn handler(
    _x: &A,
    _y: &B<A>,
    _d: &D,
    _e: &E,
    _f: &F,
    _g: &G<A>,
    _h1: &H<A>,
    _h2: &H<E>,
) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.route(GET, "/handler", f!(crate::handler));
    bp
}
