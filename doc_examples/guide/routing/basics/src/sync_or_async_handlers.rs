use pavex::http::StatusCode;

/// An asynchronous request handler.
pub async fn greet() -> StatusCode {
    StatusCode::OK
}

/// A synchronous request handler.
pub fn greet_sync() -> StatusCode {
    StatusCode::OK
}
