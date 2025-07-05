//! px:extraction
use pavex::http::StatusCode;
use pavex::post;
use pavex::request::body::UrlEncodedBody;

// px:struct_def:start
#[derive(serde::Deserialize)] // px:struct_def:hl
pub struct HomeListing {
    address: String,
    price: u64,
}
// px:struct_def:end

#[post(path = "/search")]
pub fn search_form(body: &UrlEncodedBody<HomeListing> /* px::ann:1 */) -> StatusCode {
    println!(
        "New home listing at {}, for ${}",
        body.0.address, body.0.price
    );
    StatusCode::OK // px::skip
}
