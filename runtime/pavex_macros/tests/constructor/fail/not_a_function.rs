use pavex_macros::request_scoped;

pub struct A;

impl A {
    #[request_scoped]
    const A: usize = 42;
}

fn main() {}
