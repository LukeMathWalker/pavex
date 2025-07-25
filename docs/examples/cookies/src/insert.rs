//! px:insert
use pavex::Response;
use pavex::cookie::{ResponseCookie, ResponseCookies};
use pavex::get;
use pavex::time::Zoned;

#[get(path = "/")]
pub fn insert_cookie(response_cookies: &mut ResponseCookies) -> Response {
    let now = Zoned::now().to_string();
    let cookie = ResponseCookie::new("last_visited", now)
        // We restrict the cookie to a specific path.
        .set_path("/web");

    // Make sure to insert the cookie into `&mut ResponseCookies`!
    // Otherwise, the cookie won't be attached to the response.
    response_cookies.insert(cookie);

    // You don't have to manually attach the cookie to the response!
    // It'll be done by the injector middleware at the end of the request
    // processing pipeline.
    Response::ok()
}
