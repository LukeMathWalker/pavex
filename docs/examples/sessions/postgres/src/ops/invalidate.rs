//! px:invalidate
use anyhow::Error;
use pavex::Response;
use pavex_session::Session;

#[pavex::get(path = "/invalidate")]
pub async fn invalidate(session: &mut Session<'_>) -> Result<Response, Error> {
    session.invalidate(); // px::hl
    Ok(Response::ok()) // px::skip
}
