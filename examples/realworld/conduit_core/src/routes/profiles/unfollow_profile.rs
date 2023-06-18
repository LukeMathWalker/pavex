use pavex::{extract::route::RouteParams, http::StatusCode};

use crate::schemas::Profile;

#[RouteParams]
#[derive(Debug)]
pub struct UnfollowProfile {
    pub username: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnfollowProfileResponse {
    pub profile: Profile,
}

pub fn unfollow_profile(_params: RouteParams<UnfollowProfile>) -> StatusCode {
    StatusCode::OK
}
