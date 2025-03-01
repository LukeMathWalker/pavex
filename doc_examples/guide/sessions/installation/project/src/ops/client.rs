use anyhow::Error;
use pavex::response::Response;
use pavex_session::Session;

pub async fn handler(session: &mut Session<'_>) -> Result<Response, Error> {
    let key = "user.id";
    let value = "my-user-identifier";

    // Insertion
    session.client_mut().insert(key, value)?;

    // Retrieval
    let stored: Option<String> = session.client().get(key)?;
    assert_eq!(stored.as_deref(), Some(value));

    // Removal
    session.client_mut().remove_raw(key);
    assert_eq!(session.client().get_raw(key), None);

    Ok(Response::ok())
}
