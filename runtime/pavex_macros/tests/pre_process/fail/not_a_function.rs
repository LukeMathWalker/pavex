use pavex_macros::pre_process;

pub struct A;

impl A {
    #[pre_process]
    const A: usize = 42;
}

fn main() {}
