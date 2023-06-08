use pavex_runtime::{hyper::StatusCode, extract::route::RouteParams};

#[derive(Debug, serde::Deserialize)]
pub struct FavoriteArticle {
    pub slug: String,
}

pub fn favorite_article(_params: RouteParams<FavoriteArticle>) -> StatusCode {
    StatusCode::OK
}