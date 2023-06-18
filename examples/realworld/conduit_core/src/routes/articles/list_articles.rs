use pavex::{extract::query::QueryParams, hyper::StatusCode};

use crate::schemas::Article;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListArticles {
    pub tag: Option<String>,
    pub author: Option<String>,
    pub favorited: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default = "default_offset")]
    pub offset: u64,
}

fn default_limit() -> u64 {
    20
}

fn default_offset() -> u64 {
    0
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListArticlesResponse {
    pub articles: Vec<Article>,
    pub articles_count: u64,
}

pub fn list_articles(_params: QueryParams<ListArticles>) -> StatusCode {
    StatusCode::OK
}
