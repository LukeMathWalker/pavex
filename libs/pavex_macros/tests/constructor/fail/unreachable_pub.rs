mod private {
    use pavex_macros::singleton;

    pub struct A;

    #[singleton(id = "A_")]
    pub fn a() -> A {
        todo!()
    }
}

fn main() {}
