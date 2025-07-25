use pavex::{
    Response,
    cookie::{Processor, ResponseCookies},
    post_process,
};
use tracing::Span;

use crate::{Session, errors::FinalizeError};

/// A post-processing middleware to attach a session cookie to the outgoing response, if needed.
///
/// It will also sync the session server-side state with the chosen storage backend.
#[tracing::instrument(
    name = "Finalize session",
    level = tracing::Level::DEBUG, skip_all,
    fields(session.cookie.set = tracing::field::Empty)
)]
#[post_process]
pub async fn finalize_session<'store>(
    response: Response,
    response_cookies: &mut ResponseCookies,
    // TODO: we'll use the processor to make sure that the outgoing
    //  session cookie is:
    //  - encrypted, if the client state is not empty
    //  - signed, otherwise
    //  But adding it now to avoid breaking changes later.
    _processor: &Processor,
    mut session: Session<'store>,
) -> Result<Response, FinalizeError> {
    let cookie = session.finalize().await?;

    Span::current().record("session.cookie.set", cookie.is_some());

    if let Some(cookie) = cookie {
        response_cookies.insert(cookie);
    }

    Ok(response)
}
