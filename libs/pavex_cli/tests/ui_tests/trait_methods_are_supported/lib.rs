use pavex_builder::{f, AppBlueprint, Lifecycle};

pub struct A;

pub struct B;

pub struct C;

pub trait MyTrait {
    fn a_method_that_returns_self() -> Self;
    fn a_method_that_borrows_self(&self) -> B;
    fn a_method_that_consumes_self(self) -> C;
}

impl MyTrait for A {
    fn a_method_that_returns_self() -> Self {
        todo!()
    }
    fn a_method_that_borrows_self(&self) -> B {
        todo!()
    }
    fn a_method_that_consumes_self(self) -> C {
        todo!()
    }
}

pub fn handler(_b: B, _c: C) -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.constructor(
        f!(<crate::A as crate::MyTrait>::a_method_that_returns_self),
        Lifecycle::RequestScoped,
    );
    bp.constructor(
        f!(<crate::A as crate::MyTrait>::a_method_that_borrows_self),
        Lifecycle::RequestScoped,
    );
    bp.constructor(
        f!(<crate::A as crate::MyTrait>::a_method_that_consumes_self),
        Lifecycle::RequestScoped,
    );
    bp.route(f!(crate::handler), "/home");
    bp
}
