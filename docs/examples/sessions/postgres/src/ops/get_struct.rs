//! px:server_get_struct
use anyhow::Error;
use pavex::response::Response;
use pavex_session::Session;

#[derive(serde::Serialize, serde::Deserialize)] // px::ann:1 px::hl
struct AuthInfo {
    user_id: String,
    email: String,
}

#[pavex::get(path = "/get_struct")]
pub async fn get_struct(session: &Session<'_>) -> Result<Response, Error> {
    let auth_info: Option<AuthInfo> /* px::ann:2 */ = session.get("user").await?; // px::hl
    Ok(Response::ok()) // px::skip
}
