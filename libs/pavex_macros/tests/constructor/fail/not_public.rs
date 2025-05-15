use pavex_macros::{constructor, request_scoped, singleton, transient};

pub struct A;

#[singleton]
fn a1() -> A {
    todo!()
}

#[constructor(singleton)]
fn a2() -> A {
    todo!()
}

#[transient]
fn a3() -> A {
    todo!()
}

#[request_scoped]
fn a4() -> A {
    todo!()
}

pub struct B;

#[singleton]
pub(crate) fn b1() -> B {
    todo!()
}

#[constructor(singleton)]
pub(crate) fn b2() -> B {
    todo!()
}

#[transient]
pub(crate) fn b3() -> B {
    todo!()
}

#[request_scoped]
pub(crate) fn b4() -> B {
    todo!()
}

fn main() {}
