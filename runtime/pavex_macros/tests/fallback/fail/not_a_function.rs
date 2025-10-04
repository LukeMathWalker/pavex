use pavex_macros::fallback;

pub struct A;

impl A {
    #[fallback]
    const A: usize = 42;
}

fn main() {}
