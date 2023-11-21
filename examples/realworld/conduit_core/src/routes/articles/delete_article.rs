use pavex::{extract::route::RouteParams, http::StatusCode};

#[derive(Debug)]
#[RouteParams]
pub struct DeleteArticle {
    pub slug: String,
}

pub fn delete_article(_params: RouteParams<DeleteArticle>) -> StatusCode {
    StatusCode::OK
}
