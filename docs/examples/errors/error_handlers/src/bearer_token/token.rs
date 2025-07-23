//! px:fallible_constructor
use super::BearerExtractionError;
use pavex::{methods, request::RequestHead};

pub struct BearerToken(String);

#[methods]
impl BearerToken {
    /// Extract a bearer token from the `Authorization` header attached
    /// to the incoming request.
    #[request_scoped]
    pub fn extract(head: &RequestHead) -> Result<Self, BearerExtractionError /* px::ann:1 */> {
        todo!() // px::skip
    }
}
