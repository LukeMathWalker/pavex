use pavex::cookie::RequestCookies;
use pavex::response::Response;

pub fn handler(request_cookies: &RequestCookies) -> Response {
    let Some(session_id) = request_cookies.get("session_id") else {
        return Response::unauthorized();
    };
    // Further processing
    Response::ok()
}
