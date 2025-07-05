//! px:multiple_named_parameters
use pavex::get;
use pavex::request::path::PathParams;
use pavex::response::Response;

#[PathParams]
pub struct Info {
    pub first_name: String,
    pub last_name: String,
}

#[get(path = "/greet/{first_name}/{last_name}")] // px::hl
pub fn formal_greet(info: PathParams<Info>) -> Response {
    let body = format!("Hello, {} {}!", info.0.first_name, info.0.last_name);
    Response::ok().set_typed_body(body)
}
