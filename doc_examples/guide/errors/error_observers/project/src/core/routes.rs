use pavex::http::StatusCode;

pub fn handler() -> Result<StatusCode, anyhow::Error> {
    todo!()
}

pub fn error2response(e: &anyhow::Error) -> StatusCode {
    todo!()
}
