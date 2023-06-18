use pavex::{extract::route::RouteParams, http::StatusCode};

use crate::schemas::Profile;

#[derive(Debug)]
#[RouteParams]
pub struct GetProfile {
    pub username: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetProfileResponse {
    pub profile: Profile,
}

pub fn get_profile(_params: RouteParams<GetProfile>) -> StatusCode {
    StatusCode::OK
}
