use pavex::http::StatusCode;
use pavex::request::route::RawRouteParams;

pub fn handler(params: &RawRouteParams) -> StatusCode {
    for (name, value) in params.iter() {
        println!("The route parameter `{name}` was set to `{value}`");
    }
    StatusCode::OK
}
