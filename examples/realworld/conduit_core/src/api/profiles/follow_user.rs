use pavex::{extract::route::RouteParams, http::StatusCode};

#[derive(Debug)]
#[RouteParams]
pub struct FollowUser {
    pub username: String,
}

pub fn follow_user(_params: RouteParams<FollowUser>) -> StatusCode {
    StatusCode::OK
}
