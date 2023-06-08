use pavex_runtime::{extract::{body::JsonBody, route::RouteParams}, hyper::StatusCode};

#[derive(Debug, serde::Deserialize)]
pub struct PublishCommentRoute {
    pub slug: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct PublishCommentBody {
    pub comment: CommentDraft,
}

#[derive(Debug, serde::Deserialize)]
pub struct CommentDraft {
    pub body: String,
}

pub fn publish_comment(
    _route: RouteParams<PublishCommentRoute>,
    _body: JsonBody<PublishCommentBody>,
) -> StatusCode {
    StatusCode::OK
}
