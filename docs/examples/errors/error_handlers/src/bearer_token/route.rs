use super::BearerToken;
use pavex::Response;

#[pavex::post(path = "/bearer")]
pub fn auth(token: &BearerToken) -> Response {
    todo!()
}
