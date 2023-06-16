use pavex::{extract::body::JsonBody, http::StatusCode};
use secrecy::Secret;

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

pub fn signup(_body: JsonBody<Signup>) -> StatusCode {
    StatusCode::OK
}
