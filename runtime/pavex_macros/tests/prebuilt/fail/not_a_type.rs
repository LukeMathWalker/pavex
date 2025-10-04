use pavex_macros::prebuilt;

pub struct A;

impl A {
    #[prebuilt]
    pub fn new() -> Self {
        Self {}
    }
}

fn main() {}
