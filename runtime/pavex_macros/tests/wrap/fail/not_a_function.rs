use pavex_macros::wrap;

pub struct A;

impl A {
    #[wrap]
    const A: usize = 42;
}

fn main() {}
