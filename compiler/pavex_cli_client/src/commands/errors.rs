//! Common errors for all CLI commands.
#[derive(Debug, thiserror::Error)]
#[error("Failed to invoke `{command}`")]
pub struct InvocationError {
    #[source]
    pub(crate) source: std::io::Error,
    pub(crate) command: String,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
#[error("The invocation of `{command}` was terminated by a signal")]
pub struct SignalTermination {
    pub(crate) command: String,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
#[error("The invocation of `{command}` exited with a non-zero status code: {code}")]
pub struct NonZeroExitCode {
    pub code: i32,
    pub(crate) command: String,
}
