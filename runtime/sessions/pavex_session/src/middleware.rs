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
    processor: &Processor,
    mut session: Session<'store>,
) -> Result<Response, FinalizeError> {
    // If the client-side session state is not empty, we require encryption
    // to minimize the risk of exposure for sensitive data.
    let must_encrypt = !session.client().is_empty();
    let cookie = session.finalize().await?;

    Span::current().record("session.cookie.set", cookie.is_some());

    if let Some(cookie) = cookie {
        let will_encrypt = processor.will_encrypt(cookie.name());

        if must_encrypt && !will_encrypt {
            return Err(FinalizeError::EncryptionRequired {
                cookie_name: cookie.name().to_string(),
            });
        }
        if !(will_encrypt || processor.will_sign(cookie.name())) {
            return Err(FinalizeError::CryptoRequired {
                cookie_name: cookie.name().to_string(),
            });
        }
        response_cookies.insert(cookie);
    }

    Ok(response)
}
