use pavex::{delete, http::StatusCode, request::path::PathParams};

#[derive(Debug)]
#[PathParams]
pub struct DeleteComment {
    pub slug: String,
    pub comment_id: u64,
}

#[delete(path = "/{slug}/comments/{comment_id}")]
pub fn delete_comment(_route: PathParams<DeleteComment>) -> StatusCode {
    StatusCode::OK
}
