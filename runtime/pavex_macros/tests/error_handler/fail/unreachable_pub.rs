mod private {
    use pavex_macros::error_handler;

    pub struct A;

    #[error_handler]
    pub fn handler(_e: &pavex::Error) -> A {
        todo!()
    }
}

fn main() {}
