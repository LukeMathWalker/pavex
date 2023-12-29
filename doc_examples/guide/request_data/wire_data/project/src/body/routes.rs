use pavex::http::StatusCode;
use pavex::request::body::RawIncomingBody;

pub fn handler(body: RawIncomingBody) -> StatusCode {
    // Your processing logic goes here
    StatusCode::OK
}
