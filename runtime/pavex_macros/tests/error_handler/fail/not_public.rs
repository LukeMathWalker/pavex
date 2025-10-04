use pavex_macros::error_handler;

pub struct A;

#[error_handler]
fn handler(_e: &pavex::Error) -> A {
    todo!()
}

pub struct B;

#[error_handler]
pub(crate) fn b(_e: &pavex::Error) -> B {
    todo!()
}

fn main() {}
