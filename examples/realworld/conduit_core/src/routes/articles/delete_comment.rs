use pavex::{http::StatusCode, request::path::PathParams};

#[derive(Debug)]
#[PathParams]
pub struct DeleteComment {
    pub slug: String,
    pub comment_id: u64,
}

pub fn delete_comment(_route: PathParams<DeleteComment>) -> StatusCode {
    StatusCode::OK
}
