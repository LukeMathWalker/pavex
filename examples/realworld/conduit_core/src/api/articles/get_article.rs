use pavex::{extract::route::RouteParams, hyper::StatusCode};

#[derive(Debug)]
#[RouteParams]
pub struct GetArticle {
    pub slug: String,
}

pub fn get_article(_params: RouteParams<GetArticle>) -> StatusCode {
    StatusCode::OK
}
