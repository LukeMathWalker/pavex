use pavex::{extract::route::RouteParams, hyper::StatusCode};

use crate::schemas::Comment;

#[derive(Debug)]
#[RouteParams]
pub struct ListComments {
    pub slug: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCommentsResponse {
    pub comments: Vec<Comment>,
}

pub fn list_comments(_route: RouteParams<ListComments>) -> StatusCode {
    StatusCode::OK
}
