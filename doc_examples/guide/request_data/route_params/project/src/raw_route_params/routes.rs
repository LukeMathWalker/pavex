use pavex::http::StatusCode;
use pavex::request::route::RawRouteParams;

pub fn handler(params: &RawRouteParams) -> StatusCode {
    for (name, value) in params.iter() {
        println!("`{name}` was set to `{}`", value.as_str());
    }
    StatusCode::OK
}
