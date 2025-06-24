use anyhow::Error;
use pavex::response::Response;
use pavex_session::Session;

pub async fn handler(session: &mut Session<'_>) -> Result<Response, Error> {
    let user_id: Option<String> /* (1)! */ = session.remove("user.id").await?;
    Ok(Response::ok())
}
