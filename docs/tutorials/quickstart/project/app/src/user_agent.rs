use pavex::http::header::{ToStrError, USER_AGENT};
use pavex::request::RequestHead;
use pavex::{Response, error_handler, methods};

pub enum UserAgent {
    /// No `User-Agent` header was provided.
    Unknown,
    /// The value of the `User-Agent` header for the incoming request.
    Known(String),
}

#[methods]
impl UserAgent {
    #[request_scoped]
    pub fn extract(request_head: &RequestHead) -> Result<Self, ToStrError> {
        let Some(user_agent) = request_head.headers.get(USER_AGENT) else {
            return Ok(Self::Unknown);
        };

        user_agent.to_str().map(|s| UserAgent::Known(s.into()))
    }
}

#[error_handler]
pub fn invalid_user_agent(_e: &ToStrError) -> Response {
    let body = "The `User-Agent` header value can only use ASCII printable characters.";
    Response::bad_request().set_typed_body(body)
}
