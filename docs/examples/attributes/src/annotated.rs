//! px:annotated_example
use pavex::get;
use pavex::response::Response;

#[get(path = "/")] // px::ann:1
pub fn landing_page() -> Response {
    Response::ok() // px::skip
}
