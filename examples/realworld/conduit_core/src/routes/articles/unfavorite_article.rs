use pavex::{extract::route::RouteParams, hyper::StatusCode};

use crate::schemas::Article;

#[derive(Debug)]
#[RouteParams]
pub struct UnfavoriteArticle {
    pub slug: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnfavoriteArticleResponse {
    pub article: Article,
}

pub fn unfavorite_article(_params: RouteParams<UnfavoriteArticle>) -> StatusCode {
    StatusCode::OK
}
