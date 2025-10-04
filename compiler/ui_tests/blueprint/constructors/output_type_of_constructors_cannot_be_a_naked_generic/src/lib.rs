use pavex::{blueprint::from, Blueprint};

#[pavex::request_scoped]
pub fn naked<T>() -> T {
    todo!()
}

#[pavex::request_scoped]
pub fn fallible_naked<T>() -> Result<T, FallibleError> {
    todo!()
}

pub struct FallibleError;

#[pavex::error_handler]
pub fn error_handler(_e: &FallibleError) -> pavex::Response {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler(_a: u8, _b: u16) -> pavex::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
