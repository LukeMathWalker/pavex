use pavex_runtime::{extract::route::RouteParams, http::StatusCode};

#[RouteParams]
#[derive(Debug)]
pub struct UnfollowUser {
    pub username: String,
}

pub fn unfollow_user(_params: RouteParams<UnfollowUser>) -> StatusCode {
    StatusCode::OK
}