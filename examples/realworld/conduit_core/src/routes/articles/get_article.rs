use pavex::{extract::route::RouteParams, hyper::StatusCode};

use crate::schemas::Article;

#[derive(Debug)]
#[RouteParams]
pub struct GetArticle {
    pub slug: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetArticleResponse {
    pub article: Article,
}

pub fn get_article(_params: RouteParams<GetArticle>) -> StatusCode {
    StatusCode::OK
}
