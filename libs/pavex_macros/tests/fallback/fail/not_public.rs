use pavex_macros::fallback;

pub struct A;

#[fallback]
fn a() -> A {
    todo!()
}

pub struct B;

#[fallback]
pub(crate) fn b() -> B {
    todo!()
}

fn main() {}
