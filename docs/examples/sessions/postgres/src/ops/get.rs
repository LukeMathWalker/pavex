//! px:server_get
use anyhow::Error;
use pavex::Response;
use pavex_session::Session;

#[pavex::get(path = "/get")]
pub async fn get_plain(session: &Session<'_> /* px::ann:1 */) -> Result<Response, Error> {
    let user_id: Option<String> /* px::ann:2 */ = session.get("user.id").await?; // px::ann:3 px::hl
    Ok(Response::ok()) // px::skip
}
