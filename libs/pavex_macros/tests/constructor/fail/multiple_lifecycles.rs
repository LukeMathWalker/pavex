use pavex_macros::constructor;

pub struct A;

#[constructor(singleton, request_scoped)]
pub fn new1() -> A {
    todo!()
}

#[constructor(singleton, transient)]
pub fn new2() -> A {
    todo!()
}

#[constructor(singleton, request_scoped, transient)]
pub fn new3() -> A {
    todo!()
}

#[constructor(request_scoped, transient)]
pub fn new4() -> A {
    todo!()
}

fn main() {}
