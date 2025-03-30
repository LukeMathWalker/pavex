use pavex_macros::constructor;

pub struct A;

impl A {
    #[constructor]
    const A: usize = 42;
}

fn main() {}
