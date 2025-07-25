//! px:server_insert_struct
use anyhow::Error;
use pavex::Response;
use pavex_session::Session;

#[derive(serde::Serialize, serde::Deserialize)] // px::ann:1 px::hl
struct AuthInfo {
    user_id: String,
    email: String,
}

#[pavex::get(path = "/insert_struct")]
pub async fn insert_struct(session: &mut Session<'_>) -> Result<Response, Error> {
    let info = AuthInfo {
        user_id: "my-user-identifier".into(),
        email: "user@domain.com".into(),
    };
    session.insert("user", info).await?; // px::ann:2 px::hl
    Ok(Response::ok()) // px::skip
}
