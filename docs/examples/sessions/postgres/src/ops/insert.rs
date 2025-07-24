//! px:server_insert
use anyhow::Error;
use pavex::Response;
use pavex_session::Session;

#[pavex::get(path = "/insert")]
pub async fn insert(session: &mut Session<'_> /* px::ann:1 */) -> Result<Response, Error> {
    session.insert("user.id", "my-user-identifier").await?; // px::hl
    Ok(Response::ok()) // px::skip
}
