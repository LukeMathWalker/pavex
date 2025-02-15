use pavex::http::StatusCode;

use crate::schemas::User;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetUserResponse {
    user: User,
}

pub fn get_user() -> StatusCode {
    StatusCode::OK
}
