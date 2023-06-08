use pavex_runtime::hyper::StatusCode;

pub fn ping() -> StatusCode {
    StatusCode::OK
}