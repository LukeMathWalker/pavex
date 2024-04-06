use pavex::cookie::RequestCookies;
use pavex::response::Response;

pub fn handler(request_cookies: &RequestCookies) -> Response {
    let cookies_values: Vec<_> = match request_cookies.get_all("origin") {
        Some(cookies) => cookies.values().collect(),
        None => Vec::new(),
    };
    // Further processing
    Response::ok()
}
