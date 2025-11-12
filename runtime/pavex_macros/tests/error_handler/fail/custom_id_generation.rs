use pavex_macros::error_handler;

#[error_handler(id = "CUSTOM")]  
pub fn invalid_handler(_a: &str, _b: u64) -> pavex::Response {
    todo!()
}

fn test_handle_exists() {
    let _handle = CUSTOM;
}

fn main() {}