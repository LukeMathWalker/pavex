//! px:extraction
use pavex::http::StatusCode;
use pavex::post;
use pavex::request::body::JsonBody;

// px:struct_def:start
#[derive(serde::Deserialize)] // px:struct_def:hl
pub struct HomeListing {
    address: String,
    price: u64,
}
// px:struct_def:end

#[post(path = "/listing")]
pub fn create_listing(body: &JsonBody<HomeListing> /* px::ann:1 */) -> StatusCode {
    println!(
        "The home you want to sell for ${} is located in {}",
        body.0.price, body.0.address
    );
    StatusCode::OK // px::skip
}
