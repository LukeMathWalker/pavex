use pavex_runtime::{extract::route::RouteParams, hyper::StatusCode};

#[derive(Debug, serde::Deserialize)]
pub struct GetArticle {
    pub slug: String,
}

pub fn get_article(_params: RouteParams<GetArticle>) -> StatusCode {
    StatusCode::OK
}