pub async fn error_logger(e: &pavex::Error, root_span: &RootSpan /* (1)! */) {
    root_span.record("error.msg", tracing::field::display(e));
    root_span.record("error.details", tracing::field::debug(e));
}

use crate::di::RootSpan;
