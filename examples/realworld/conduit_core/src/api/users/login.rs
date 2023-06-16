use pavex::{extract::body::JsonBody, http::StatusCode};
use secrecy::Secret;

#[derive(serde::Deserialize)]
pub struct LoginUser {
    pub user: UserCredentials,
}

#[derive(serde::Deserialize)]
pub struct UserCredentials {
    pub email: String,
    pub password: Secret<String>,
}

pub fn login(_body: JsonBody<LoginUser>) -> StatusCode {
    StatusCode::OK
}
