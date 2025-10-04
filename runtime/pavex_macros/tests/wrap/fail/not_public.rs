use pavex_macros::wrap;

pub struct A;

#[wrap]
fn a() -> A {
    todo!()
}

pub struct B;

#[wrap]
pub(crate) fn b() -> B {
    todo!()
}

fn main() {}
