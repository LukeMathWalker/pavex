mod private {
    use pavex_macros::singleton;

    pub struct A;

    #[singleton]
    pub fn a() -> A {
        todo!()
    }
}

fn main() {}
