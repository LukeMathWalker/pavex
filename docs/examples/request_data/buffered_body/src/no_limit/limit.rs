//! px:no_limit
use pavex::request::body::BodySizeLimit;
use pavex::request_scoped;

#[request_scoped]
pub fn no_limit() -> BodySizeLimit {
    BodySizeLimit::Disabled
}
