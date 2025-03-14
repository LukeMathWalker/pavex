use pavex_macros::constructor;

pub struct A;

impl A {
    #[constructor]
    pub fn new() -> Self {
        A
    }
}

fn main() {}
