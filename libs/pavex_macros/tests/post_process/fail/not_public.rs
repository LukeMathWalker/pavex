use pavex_macros::post_process;

pub struct A;

#[post_process]
fn a() -> A {
    todo!()
}

pub struct B;

#[post_process]
pub(crate) fn b() -> B {
    todo!()
}

fn main() {}
