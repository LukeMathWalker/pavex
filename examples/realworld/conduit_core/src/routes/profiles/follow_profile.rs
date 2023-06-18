use pavex::{extract::route::RouteParams, http::StatusCode};

use crate::schemas::Profile;

#[derive(Debug)]
#[RouteParams]
pub struct FollowProfile {
    pub username: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowProfileResponse {
    pub profile: Profile,
}

pub fn follow_profile(_params: RouteParams<FollowProfile>) -> StatusCode {
    StatusCode::OK
}
