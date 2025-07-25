use pavex::{get, http::StatusCode, request::path::PathParams};

use crate::schemas::Comment;

#[derive(Debug)]
#[PathParams]
pub struct ListComments {
    pub slug: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCommentsResponse {
    pub comments: Vec<Comment>,
}

#[get(path = "/{slug}/comments")]
pub fn list_comments(_route: PathParams<ListComments>) -> StatusCode {
    StatusCode::OK
}
