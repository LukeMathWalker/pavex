use pavex::{blueprint::from, Blueprint};

pub struct A;

pub struct B;

pub struct C;

pub struct F;

#[pavex::methods]
// A foreign trait, from `std`.
impl Default for F {
    #[request_scoped]
    fn default() -> Self {
        todo!()
    }
}

pub trait MyTrait {
    fn a_method_that_returns_self() -> Self;
    fn a_method_that_borrows_self(&self) -> B;
}

pub trait AnotherTrait {
    fn a_method_that_consumes_self(self) -> C;
}

#[pavex::methods]
impl MyTrait for A {
    #[request_scoped]
    fn a_method_that_returns_self() -> Self {
        todo!()
    }
    #[request_scoped]
    fn a_method_that_borrows_self(&self) -> B {
        todo!()
    }
}

#[pavex::methods]
impl AnotherTrait for B {
    #[request_scoped]
    fn a_method_that_consumes_self(self) -> C {
        todo!()
    }
}

#[pavex::get(path = "/")]
pub fn handler(_a: A, _c: C, _f: F) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
