use pavex_macros::config;

pub struct A;

impl A {
    #[config]
    pub fn new() -> Self {
        Self {}
    }
}

fn main() {}
