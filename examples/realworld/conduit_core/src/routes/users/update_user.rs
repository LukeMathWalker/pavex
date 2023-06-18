use pavex::{extract::body::JsonBody, http::StatusCode};
use secrecy::Secret;

use crate::schemas::User;

#[derive(serde::Deserialize)]
pub struct UpdateUser {
    pub user: UpdatedDetails,
}

#[derive(serde::Deserialize)]
pub struct UpdatedDetails {
    pub email: Option<String>,
    pub username: Option<String>,
    pub password: Option<Secret<String>>,
    pub bio: Option<String>,
    pub image: Option<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserResponse {
    pub user: User,
}

pub fn update_user(_body: JsonBody<UpdateUser>) -> StatusCode {
    StatusCode::OK
}
