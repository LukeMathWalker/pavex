use pavex_runtime::{extract::query::QueryParams, hyper::StatusCode};

#[derive(Debug, serde::Deserialize)]
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

pub fn list_articles(_params: QueryParams<ListArticles>) -> StatusCode {
    StatusCode::OK
}