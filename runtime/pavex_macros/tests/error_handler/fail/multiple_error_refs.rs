use pavex_macros::error_handler;

#[error_handler]
pub fn handler(#[px(error_ref)] _a: &str, #[px(error_ref)] _b: u64) -> pavex::Response {
    todo!()
}

fn main() {}
