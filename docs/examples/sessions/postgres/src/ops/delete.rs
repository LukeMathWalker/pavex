//! px:server_delete
use anyhow::Error;
use pavex::Response;
use pavex_session::Session;

#[pavex::delete(path = "/delete")]
pub async fn delete_session(session: &mut Session<'_>) -> Result<Response, Error> {
    session.delete(); // px::hl
    Ok(Response::ok()) // px::skip
}
