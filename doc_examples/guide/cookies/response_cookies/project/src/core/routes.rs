use pavex::cookie::{ResponseCookie, ResponseCookies};
use pavex::response::Response;
use pavex::time::{format_description::well_known::Iso8601, OffsetDateTime};

pub fn handler(response_cookies: &mut ResponseCookies) -> Response {
    let now = OffsetDateTime::now_utc().format(&Iso8601::DEFAULT).unwrap();
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
