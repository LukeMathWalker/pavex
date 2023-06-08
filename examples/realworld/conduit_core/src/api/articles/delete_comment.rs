use pavex_runtime::{extract::route::RouteParams, hyper::StatusCode};

#[derive(Debug, serde::Deserialize)]
pub struct DeleteComment {
    pub slug: String,
    pub comment_id: u64
}

pub fn delete_comment(
    _route: RouteParams<DeleteComment>,
) -> StatusCode {
    StatusCode::OK
}