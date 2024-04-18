use pavex::http::StatusCode;
use pavex::request::body::UrlEncodedBody;

#[derive(serde::Deserialize)]
pub struct HomeListing {
    address: String,
    price: u64,
}

pub fn handler(params: &UrlEncodedBody<HomeListing>) -> StatusCode {
    StatusCode::OK
}
