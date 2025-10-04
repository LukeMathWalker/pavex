use pavex_macros::error_handler;

#[error_handler]
pub fn handler(_a: &str, _b: u64) -> pavex::Response {
    todo!()
}

fn main() {}
