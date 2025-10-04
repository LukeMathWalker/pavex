use pavex_macros::error_observer;

pub struct A;

impl A {
    #[error_observer]
    const A: usize = 42;
}

fn main() {}
