use pavex_macros::constructor;

pub struct A;

impl A {
    #[constructor(lifecycle = "request")]
    pub fn new() -> Self {
        A
    }
}

fn main() {}
