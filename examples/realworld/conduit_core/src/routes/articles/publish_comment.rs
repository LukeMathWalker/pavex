use pavex::{
    http::StatusCode,
    request::{body::JsonBody, path::PathParams},
};

use crate::schemas::Comment;

#[derive(Debug)]
#[PathParams]
pub struct PublishCommentRoute {
    pub slug: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishCommentBody {
    pub comment: CommentDraft,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentDraft {
    pub body: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishCommentResponse {
    pub comment: Comment,
}

pub fn publish_comment(
    _route: PathParams<PublishCommentRoute>,
    _body: JsonBody<PublishCommentBody>,
) -> StatusCode {
    StatusCode::OK
}
