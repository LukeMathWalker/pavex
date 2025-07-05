//! px:fallible
use crate::errors::AuthError;
use pavex::middleware::Processing;
use pavex::pre_process;
use pavex::request::RequestHead;

#[pre_process]
pub fn reject_anonymous(request: &RequestHead) -> Result<Processing, AuthError> {
    if request.headers.get("Authorization").is_none() {
        return Err(AuthError);
    }
    Ok(Processing::Continue)
}
