use pavex::methods;
use std::ops::Deref;

pub struct RootSpan(tracing::Span);

#[methods]
impl RootSpan {
    #[request_scoped]
    pub fn new() -> Self {
        Self(tracing::info_span!(
            "Request",
            error.details = tracing::field::Empty,
            error.msg = tracing::field::Empty,
        ))
    }
}

impl Deref for RootSpan {
    type Target = tracing::Span;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
