use pavex::{extract::route::RouteParams, hyper::StatusCode};

#[derive(Debug)]
#[RouteParams]
pub struct FavoriteArticle {
    pub slug: String,
}

pub fn favorite_article(_params: RouteParams<FavoriteArticle>) -> StatusCode {
    StatusCode::OK
}
