//! px:handler
use super::{A, B};
use pavex::get;
use pavex::response::Response;

#[get(path = "/")]
pub fn handler(a: A, b: B) -> Response {
    todo!() // px::skip
}
