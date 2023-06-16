use pavex::{extract::route::RouteParams, hyper::StatusCode};

#[derive(Debug)]
#[RouteParams]
pub struct ListComments {
    pub slug: String,
}

pub fn list_comments(_route: RouteParams<ListComments>) -> StatusCode {
    StatusCode::OK
}
