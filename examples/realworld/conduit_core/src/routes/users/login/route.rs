use pavex::{extract::body::JsonBody, http::StatusCode};
use secrecy::Secret;
use sqlx::PgPool;

use crate::schemas::User;

use super::password;

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

pub async fn login(body: JsonBody<LoginUser>, db_pool: &PgPool) -> StatusCode {
    let UserCredentials { email, password } = body.0.user;
    password::validate_credentials(&email, password, db_pool).await;
    StatusCode::OK
}
