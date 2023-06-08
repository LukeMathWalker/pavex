use pavex_runtime::{extract::query::QueryParams, hyper::StatusCode};

#[derive(Debug, serde::Deserialize)]
pub struct GetFeed {
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

pub fn get_feed(_params: QueryParams<GetFeed>) -> StatusCode {
    StatusCode::OK
}