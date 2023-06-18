use pavex::{extract::route::RouteParams, hyper::StatusCode};

use crate::schemas::Article;

#[derive(Debug)]
#[RouteParams]
pub struct FavoriteArticle {
    pub slug: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteArticleResponse {
    pub article: Article,
}

pub fn favorite_article(_params: RouteParams<FavoriteArticle>) -> StatusCode {
    StatusCode::OK
}
