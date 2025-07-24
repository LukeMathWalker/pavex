//! px:get_one
use pavex::Response;
use pavex::cookie::RequestCookies;
use pavex::get;

#[get(path = "/")]
pub fn get_one(request_cookies: &RequestCookies) -> Response {
    let Some(session_id) = request_cookies.get("session_id") else {
        return Response::unauthorized();
    };
    Response::ok() // px::skip
}
