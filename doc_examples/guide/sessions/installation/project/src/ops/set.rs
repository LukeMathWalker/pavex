use anyhow::Error;
use pavex::response::Response;
use pavex_session::Session;

pub async fn handler(session: &mut Session<'_> /* (1)! */) -> Result<Response, Error> {
    let auth_id = "my-user-identifier";
    session.server_mut().set("user.id".into(), auth_id).await?;
    Ok(Response::ok())
}
