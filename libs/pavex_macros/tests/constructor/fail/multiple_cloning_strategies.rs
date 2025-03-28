use pavex_macros::{constructor, request_scoped, singleton, transient};

pub struct A;

#[constructor(clone_if_necessary, never_clone)]
pub fn new1() -> A {
    todo!()
}

#[singleton(clone_if_necessary, never_clone)]
pub fn new2() -> A {
    todo!()
}

#[request_scoped(clone_if_necessary, never_clone)]
pub fn new3() -> A {
    todo!()
}

#[transient(clone_if_necessary, never_clone)]
pub fn new4() -> A {
    todo!()
}

fn main() {}
