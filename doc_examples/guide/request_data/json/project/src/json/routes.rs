use pavex::http::StatusCode;
use pavex::request::body::JsonBody;

#[derive(serde::Deserialize)]
pub struct HomeListing {
    address: String,
    price: u64,
}

pub fn handler(params: &JsonBody<HomeListing>) -> StatusCode {
    StatusCode::OK
}
