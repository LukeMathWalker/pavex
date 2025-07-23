use pavex::{http::StatusCode, post, request::path::PathParams};

use crate::schemas::Article;

#[derive(Debug)]
#[PathParams]
pub struct FavoriteArticle {
    pub slug: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteArticleResponse {
    pub article: Article,
}

#[post(path = "/{slug}/favorite")]
pub fn favorite_article(_params: PathParams<FavoriteArticle>) -> StatusCode {
    StatusCode::OK
}
