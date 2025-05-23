use pavex_macros::{error_handler, methods};

#[error_handler]
pub fn handle_str(_s: &str) {}

pub struct A;

#[methods]
impl A {
    #[error_handler]
    pub fn handle_str(_s: &str) {}
}

fn main() {
    handle_str("yo");
    A::handle_str("yo");
}
