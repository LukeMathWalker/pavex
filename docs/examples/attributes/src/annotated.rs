//! px:annotated_example
use pavex::Response;
use pavex::get;

#[get(path = "/")] // px::ann:1
pub fn landing_page() -> Response {
    Response::ok() // px::skip
}
