mod private {
    use pavex_macros::error_observer;

    pub struct A;

    #[error_observer(id = "A_")]
    pub fn a() -> A {
        todo!()
    }
}

fn main() {}
