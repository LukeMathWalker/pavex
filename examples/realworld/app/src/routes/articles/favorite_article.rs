use pavex::{http::StatusCode, request::path::PathParams};

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

pub fn favorite_article(_params: PathParams<FavoriteArticle>) -> StatusCode {
    StatusCode::OK
}
