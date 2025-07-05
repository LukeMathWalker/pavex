//! px:server_remove_raw
use anyhow::Error;
use pavex::response::Response;
use pavex_session::Session;

#[pavex::get(path = "/remove_raw")]
pub async fn remove_raw(session: &mut Session<'_>) -> Result<Response, Error> {
    session.remove_raw("user.id").await?; // px::hl
    Ok(Response::ok()) // px::skip
}
