//! px:server_clear
use anyhow::Error;
use pavex::response::Response;
use pavex_session::Session;

#[pavex::get(path = "/clear")]
pub async fn clear_session(session: &mut Session<'_>) -> Result<Response, Error> {
    session.clear().await?; // px::hl
    Ok(Response::ok()) // px::skip
}
