//! px:raw
use pavex::get;
use pavex::http::StatusCode;
use pavex::request::path::RawPathParams;

#[get(path = "/room/{id}")]
pub fn raw(params: &RawPathParams) -> StatusCode {
    for (name, value) in params.iter() {
        println!("`{name}` was set to `{}`", value.as_str());
    }
    StatusCode::OK // px::skip
}
