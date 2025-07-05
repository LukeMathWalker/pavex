//! px:named_parameter
use pavex::get;
use pavex::request::path::PathParams;
use pavex::response::Response;

#[PathParams]
pub struct Info {
    pub name: String,
}

#[get(path = "/greet/{name}")] // px::hl
pub fn informal_greet(info: PathParams<Info>) -> Response {
    let body = format!("Hello, {}!", info.0.name);
    Response::ok().set_typed_body(body)
}
