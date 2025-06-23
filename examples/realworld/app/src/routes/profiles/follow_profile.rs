use pavex::{http::StatusCode, post, request::path::PathParams};

use crate::schemas::Profile;

#[derive(Debug)]
#[PathParams]
pub struct FollowProfile {
    pub username: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowProfileResponse {
    pub profile: Profile,
}

#[post(path = "/{username}/follow")]
pub fn follow_profile(_params: PathParams<FollowProfile>) -> StatusCode {
    StatusCode::OK
}
