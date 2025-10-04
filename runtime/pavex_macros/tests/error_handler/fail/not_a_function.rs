use pavex_macros::error_handler;

pub struct A;

impl A {
    #[error_handler]
    const A: usize = 42;
}

fn main() {}
