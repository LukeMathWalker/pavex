pub async fn error_logger(e: &pavex::Error) {
    tracing::error!(
        error.msg = %e,
        error.details = ?e,
        "An error occurred"
    );
}
