//! px:cycle_id
use anyhow::Error;
use pavex::Response;
use pavex_session::Session;

#[pavex::get(path = "/cycle_id")]
pub async fn cycle_id(session: &mut Session<'_>) -> Result<Response, Error> {
    session.cycle_id(); // px::hl
    Ok(Response::ok()) // px::skip
}
