use pavex::middleware::Processing;
use pavex::request::RequestHead;
use super::AuthError;

pub async fn reject_anonymous(request: &RequestHead) -> Result<Processing, AuthError>
{
    if request.headers.get("Authorization").is_none() {
        return Err(AuthError);
    }
    Ok(Processing::Continue)
}
