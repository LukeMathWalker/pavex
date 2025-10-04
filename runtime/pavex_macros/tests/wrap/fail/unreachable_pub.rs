mod private {
    use pavex_macros::wrap;

    pub struct A;

    #[wrap(id = "A_")]
    pub fn a() -> A {
        todo!()
    }
}

fn main() {}
