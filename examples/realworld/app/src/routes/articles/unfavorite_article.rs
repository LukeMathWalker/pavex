use pavex::{delete, http::StatusCode, request::path::PathParams};

use crate::schemas::Article;

#[derive(Debug)]
#[PathParams]
pub struct UnfavoriteArticle {
    pub slug: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnfavoriteArticleResponse {
    pub article: Article,
}

#[delete(path = "/{slug}/favorite")]
pub fn unfavorite_article(_params: PathParams<UnfavoriteArticle>) -> StatusCode {
    StatusCode::OK
}
