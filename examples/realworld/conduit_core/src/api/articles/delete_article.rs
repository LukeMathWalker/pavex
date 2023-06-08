use pavex_runtime::{extract::route::RouteParams, hyper::StatusCode};

#[derive(Debug, serde::Deserialize)]
pub struct DeleteArticle {
    pub slug: String,
}


pub fn delete_article(_params: RouteParams<DeleteArticle>) -> StatusCode {
    StatusCode::OK
}