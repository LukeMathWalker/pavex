use pavex::get;
use pavex::http::StatusCode;

/// Respond with a `200 OK` status code to indicate that the server is alive
/// and ready to accept new requests.
#[get(path = "/ping")]
pub fn ping() -> StatusCode {
    StatusCode::OK
}
