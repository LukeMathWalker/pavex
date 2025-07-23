//! px:delete
use pavex::cookie::{RemovalCookie, ResponseCookies};
use pavex::get;
use pavex::response::Response;

#[get(path = "/")]
pub fn delete_cookie(response_cookies: &mut ResponseCookies) -> Response {
    let cookie = RemovalCookie::new("last_visited")
        // We need to match the path of the cookie we want to delete.
        .set_path("/web");
    response_cookies.insert(cookie);

    Response::ok()
}
