use pavex::{http::StatusCode, request::path::PathParams};

#[derive(Debug)]
#[PathParams]
pub struct DeleteArticle {
    pub slug: String,
}

pub fn delete_article(_params: PathParams<DeleteArticle>) -> StatusCode {
    StatusCode::OK
}
