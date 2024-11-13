use pavex::http::StatusCode;

pub fn index() -> StatusCode {
    StatusCode::OK
}

pub fn unknown_domain() -> StatusCode {
    StatusCode::NOT_FOUND
}

pub fn users() -> StatusCode {
    StatusCode::OK
}
