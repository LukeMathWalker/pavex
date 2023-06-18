use pavex::{extract::body::JsonBody, http::StatusCode};
use secrecy::Secret;

use crate::schemas::User;

#[derive(serde::Deserialize)]
pub struct LoginUser {
    pub user: UserCredentials,
}

#[derive(serde::Deserialize)]
pub struct UserCredentials {
    pub email: String,
    pub password: Secret<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginUserResponse {
    pub user: User,
}

pub fn login(_body: JsonBody<LoginUser>) -> StatusCode {
    StatusCode::OK
}
