use pavex_runtime::{extract::route::RouteParams, hyper::StatusCode};

#[derive(Debug)]
#[RouteParams]
pub struct UnfavoriteArticle {
    pub slug: String,
}

pub fn unfavorite_article(_params: RouteParams<UnfavoriteArticle>) -> StatusCode {
    StatusCode::OK
}