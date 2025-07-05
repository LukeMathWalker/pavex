//! px:server_remove
use anyhow::Error;
use pavex::response::Response;
use pavex_session::Session;

#[pavex::get(path = "/remove")]
pub async fn remove(session: &mut Session<'_>) -> Result<Response, Error> {
    let user_id: Option<String> /* px::ann:1 */ = session.remove("user.id").await?; // px::hl
    Ok(Response::ok()) // px::skip
}
