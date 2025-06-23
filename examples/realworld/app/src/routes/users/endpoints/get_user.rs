use pavex::{get, http::StatusCode};

use crate::schemas::User;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetUserResponse {
    user: User,
}

#[get(path = "/user")]
pub fn get_user() -> StatusCode {
    StatusCode::OK
}
