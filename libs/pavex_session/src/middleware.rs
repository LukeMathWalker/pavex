use pavex::{cookie::ResponseCookies, response::Response};

use crate::{state::errors::FinalizeError, Session};

/// A post-processing middleware to attach a session cookie to the outgoing response, if needed.
///
/// It will also sync the session server-side state with the chosen storage backend.
pub async fn finalize_session<'store>(
    response: Response,
    response_cookies: &mut ResponseCookies,
    mut session: Session<'store>,
) -> Result<Response, FinalizeError> {
    if let Some(cookie) = session.finalize().await? {
        response_cookies.insert(cookie);
    }

    Ok(response)
}
