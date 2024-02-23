use pavex::{http::StatusCode, request::path::PathParams};

use crate::schemas::Article;

#[derive(Debug)]
#[PathParams]
pub struct GetArticle {
    pub slug: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetArticleResponse {
    pub article: Article,
}

pub fn get_article(_params: PathParams<GetArticle>) -> StatusCode {
    StatusCode::OK
}
