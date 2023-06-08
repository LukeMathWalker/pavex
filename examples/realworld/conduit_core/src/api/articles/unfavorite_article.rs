use pavex_runtime::{extract::route::RouteParams, hyper::StatusCode};

#[derive(Debug, serde::Deserialize)]
pub struct UnfavoriteArticle {
    pub slug: String,
}

pub fn unfavorite_article(_params: RouteParams<UnfavoriteArticle>) -> StatusCode {
    StatusCode::OK
}