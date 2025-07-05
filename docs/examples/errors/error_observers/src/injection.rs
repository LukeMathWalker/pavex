//! px:injection
use crate::RootSpan;
use pavex::error_observer;

#[error_observer]
pub async fn enrich_root_span(e: &pavex::Error, root_span: &RootSpan /* (1)! */) {
    root_span.record("error.msg", tracing::field::display(e));
    root_span.record("error.details", tracing::field::debug(e));
}
