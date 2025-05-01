use pavex_macros::wrap;

pub struct A;

impl A {
    #[wrap]
    pub fn extract(v: &str) -> A {
        todo!()
    }
}

fn main() {}
