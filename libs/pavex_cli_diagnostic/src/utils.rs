use std::fmt::{Display, Formatter};

#[derive(Debug, miette::Diagnostic)]
/// A thin wrapper around `anyhow::Error` that implements `miette::Diagnostic`.
pub struct InteropError(anyhow::Error);

impl Display for InteropError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for InteropError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

pub trait AnyhowBridge {
    /// Convert an `anyhow::Error` into a an error type that implements `miette::Diagnostic`.
    fn into_miette(self) -> InteropError;
}

impl AnyhowBridge for anyhow::Error {
    fn into_miette(self) -> InteropError {
        InteropError(self)
    }
}
