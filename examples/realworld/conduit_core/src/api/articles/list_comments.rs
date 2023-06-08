use pavex_runtime::{extract::route::RouteParams, hyper::StatusCode};

#[derive(Debug, serde::Deserialize)]
pub struct ListComments {
    pub slug: String,
}

pub fn list_comments(
    _route: RouteParams<ListComments>,
) -> StatusCode {
    StatusCode::OK
}