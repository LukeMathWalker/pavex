use pavex::{cookie::ResponseCookies, response::Response};
use tracing::Span;

use crate::{state::errors::FinalizeError, Session};

/// A post-processing middleware to attach a session cookie to the outgoing response, if needed.
///
/// It will also sync the session server-side state with the chosen storage backend.
#[tracing::instrument(
    name = "Finalize session", 
    level = tracing::Level::DEBUG, skip_all, 
    fields(session.cookie.set = tracing::field::Empty)
)]
pub async fn finalize_session<'store>(
    response: Response,
    response_cookies: &mut ResponseCookies,
    mut session: Session<'store>,
) -> Result<Response, FinalizeError> {
    let cookie = session.finalize().await?;
    
    Span::current().record("session.cookie.set", cookie.is_some());
    
    if let Some(cookie) = cookie {
        response_cookies.insert(cookie);
    }
    
    Ok(response)
}
