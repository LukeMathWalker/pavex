use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;

pub struct A;

pub struct B;

pub struct C;

pub struct D;

pub struct E;

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

pub fn handler(_a: A, _c: C, _d: D, _e: E) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(
        f!(<crate::A as crate::MyTrait>::a_method_that_returns_self),
        Lifecycle::RequestScoped,
    );
    bp.constructor(
        f!(<crate::A as crate::MyTrait>::a_method_that_borrows_self),
        Lifecycle::RequestScoped,
    );
    bp.constructor(
        f!(<crate::A as crate::MyTrait>::a_method_with_a_generic::<std::string::String>),
        Lifecycle::RequestScoped,
    );
    bp.constructor(
        f!(<crate::B as crate::AnotherTrait>::a_method_that_consumes_self),
        Lifecycle::RequestScoped,
    );
    bp.constructor(
        f!(<crate::C as crate::GenericTrait<std::string::String>>::a_method),
        Lifecycle::RequestScoped,
    );
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
