mod private {
    use pavex_macros::fallback;

    pub struct A;

    #[fallback(id = "A_")]
    pub fn a() -> A {
        todo!()
    }
}

fn main() {}
