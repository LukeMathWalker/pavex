use pavex_macros::pre_process;

pub struct A;

#[pre_process]
fn a() -> A {
    todo!()
}

pub struct B;

#[pre_process]
pub(crate) fn b() -> B {
    todo!()
}

fn main() {}
