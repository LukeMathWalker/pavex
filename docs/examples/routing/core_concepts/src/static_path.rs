//! px:static_path
use pavex::Response;
use pavex::get;

#[get(path = "/greet")] // px::hl
pub fn anonymous_greet() -> Response {
    Response::ok().set_typed_body("Hello world!")
}
