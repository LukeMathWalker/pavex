use pavex::{extract::route::RouteParams, http::StatusCode};

#[derive(Debug)]
#[RouteParams]
pub struct DeleteComment {
    pub slug: String,
    pub comment_id: u64,
}

pub fn delete_comment(_route: RouteParams<DeleteComment>) -> StatusCode {
    StatusCode::OK
}
