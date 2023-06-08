use pavex_runtime::{extract::route::RouteParams, http::StatusCode};

#[derive(Debug)]
#[RouteParams]
pub struct GetUser {
    pub username: String,
}

pub fn get_user(_params: RouteParams<GetUser>) -> StatusCode {
    StatusCode::OK
}