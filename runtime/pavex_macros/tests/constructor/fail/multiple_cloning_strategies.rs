use pavex_macros::{request_scoped, singleton, transient};

pub struct A;

#[singleton(clone_if_necessary, never_clone)]
pub fn new() -> A {
    todo!()
}

#[request_scoped(clone_if_necessary, never_clone)]
pub fn new2() -> A {
    todo!()
}

#[transient(clone_if_necessary, never_clone)]
pub fn new3() -> A {
    todo!()
}

fn main() {}
