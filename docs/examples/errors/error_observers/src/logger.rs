//! px:example
//! px:definition
use pavex::error_observer;
use tracing_log_error::log_error;

#[error_observer] // px:definition:hl
pub async fn emit_error_log(e: &pavex::Error) {
    log_error!(e, "An error occurred during request processing");
}
