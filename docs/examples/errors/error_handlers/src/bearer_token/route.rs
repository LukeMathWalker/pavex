use super::BearerToken;
use pavex::response::Response;

#[pavex::post(path = "/bearer")]
pub fn auth(token: &BearerToken) -> Response {
    todo!()
}
