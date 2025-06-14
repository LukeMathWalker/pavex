use pavex::blueprint::{from, Blueprint};
use pavex::f;

pub struct A;

pub struct B;

pub struct C;

pub struct D;

pub struct E;

#[derive(Default)]
pub struct F;

pub trait MyTrait {
    fn a_method_that_returns_self() -> Self;
    fn a_method_that_borrows_self(&self) -> B;
    fn a_method_with_a_generic<T>(&self) -> D;
}

pub trait AnotherTrait {
    fn a_method_that_consumes_self(self) -> C;
}

pub trait GenericTrait<T> {
    fn a_method(&self) -> E;
}

impl MyTrait for A {
    fn a_method_that_returns_self() -> Self {
        todo!()
    }
    fn a_method_that_borrows_self(&self) -> B {
        todo!()
    }
    fn a_method_with_a_generic<T>(&self) -> D {
        todo!()
    }
}

impl AnotherTrait for B {
    fn a_method_that_consumes_self(self) -> C {
        todo!()
    }
}

impl<T> GenericTrait<T> for C {
    fn a_method(&self) -> E {
        todo!()
    }
}

pub fn handler(_a: A, _c: C, _d: D, _e: E, _f: F) -> pavex::response::Response {
    todo!()
}

#[pavex::get(path = "/")]
pub fn route_handler(_a: A, _c: C, _d: D, _e: E, _f: F) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    // A foreign trait, from `std`.
    bp.request_scoped(f!(<crate::F as std::default::Default>::default));
    bp.request_scoped(f!(<crate::A as crate::MyTrait>::a_method_that_returns_self));
    bp.request_scoped(f!(<crate::A as crate::MyTrait>::a_method_that_borrows_self));
    bp.request_scoped(f!(<crate::A as crate::MyTrait>::a_method_with_a_generic::<
        std::string::String,
    >));
    bp.request_scoped(f!(
        <crate::B as crate::AnotherTrait>::a_method_that_consumes_self
    ));
    bp.request_scoped(f!(
        <crate::C as crate::GenericTrait<std::string::String>>::a_method
    ));
    bp.routes(from![crate]);
    bp
}
