use pavex::{get, Response};

#[get(path = "/")]
pub fn index() -> Response {
    Response::ok()
}

#[get(path = "/login")]
pub fn login() -> Response {
    Response::ok()
}
