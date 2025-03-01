use anyhow::Error;
use pavex::response::Response;
use pavex_session::Session;

pub async fn handler(session: &mut Session<'_>) -> Result<Response, Error> {
    session.remove_raw("user.id").await?;
    Ok(Response::ok())
}
