use pavex::{extract::body::JsonBody, http::StatusCode};
use secrecy::Secret;

use crate::schemas::User;

#[derive(serde::Deserialize)]
pub struct Signup {
    pub user: UserDetails,
}

#[derive(serde::Deserialize)]
pub struct UserDetails {
    pub username: String,
    pub email: String,
    pub password: Secret<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignupResponse {
    pub user: User,
}

pub fn signup(_body: JsonBody<Signup>) -> StatusCode {
    StatusCode::OK
}
