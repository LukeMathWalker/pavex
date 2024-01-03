use pavex::{http::StatusCode, request::path::PathParams};

use crate::schemas::Profile;

#[PathParams]
#[derive(Debug)]
pub struct UnfollowProfile {
    pub username: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnfollowProfileResponse {
    pub profile: Profile,
}

pub fn unfollow_profile(_params: PathParams<UnfollowProfile>) -> StatusCode {
    StatusCode::OK
}
