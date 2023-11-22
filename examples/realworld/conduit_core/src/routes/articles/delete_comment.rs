use pavex::{http::StatusCode, request::route::RouteParams};

#[derive(Debug)]
#[RouteParams]
pub struct DeleteComment {
    pub slug: String,
    pub comment_id: u64,
}

pub fn delete_comment(_route: RouteParams<DeleteComment>) -> StatusCode {
    StatusCode::OK
}
