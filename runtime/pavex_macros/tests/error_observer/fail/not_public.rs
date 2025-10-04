use pavex_macros::error_observer;

pub struct A;

#[error_observer]
fn a() -> A {
    todo!()
}

pub struct B;

#[error_observer]
pub(crate) fn b() -> B {
    todo!()
}

fn main() {}
