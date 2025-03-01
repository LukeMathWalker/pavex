use anyhow::Error;
use pavex::response::Response;
use pavex_session::Session;

#[derive(serde::Serialize, serde::Deserialize)] // (1)!
struct AuthInfo {
    user_id: String,
    email: String,
}

pub async fn handler(session: &mut Session<'_>) -> Result<Response, Error> {
    let info = AuthInfo {
        user_id: "my-user-identifier".into(),
        email: "user@domain.com".into(),
    };
    session.insert("user", info).await?; // (2)!

    Ok(Response::ok())
}
