use pavex::http::StatusCode;

pub async fn async_greet() -> StatusCode {
    StatusCode::OK
}

pub fn sync_greet() -> StatusCode {
    StatusCode::OK
}
