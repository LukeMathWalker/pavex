//! px:static_path
use pavex::get;
use pavex::response::Response;

#[get(path = "/greet")] // px::hl
pub fn anonymous_greet() -> Response {
    Response::ok().set_typed_body("Hello world!")
}
