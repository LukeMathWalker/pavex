use pavex::{http::StatusCode, request::path::PathParams};

use crate::schemas::Profile;

#[derive(Debug)]
#[PathParams]
pub struct GetProfile {
    pub username: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetProfileResponse {
    pub profile: Profile,
}

pub fn get_profile(_params: PathParams<GetProfile>) -> StatusCode {
    StatusCode::OK
}
