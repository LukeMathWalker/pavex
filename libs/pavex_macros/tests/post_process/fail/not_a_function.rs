use pavex_macros::post_process;

pub struct A;

impl A {
    #[post_process]
    const A: usize = 42;
}

fn main() {}
