//! px:handler
use super::{A, B};
use pavex::Response;
use pavex::get;

#[get(path = "/")]
pub fn handler(a: A, b: B) -> Response {
    todo!() // px::skip
}
