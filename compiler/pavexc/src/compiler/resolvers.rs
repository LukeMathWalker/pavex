//! Resolution of callables and types from rustdoc information.

use crate::rustdoc::CannotGetCrateData;

// Re-export types that moved to `rustdoc_resolver` so that downstream code
// within pavexc can keep importing them from this module.
pub use rustdoc_resolver::{
    InputParameterResolutionError, OutputTypeResolutionError, SelfResolutionError,
    TypeResolutionError, UnsupportedConstGeneric,
};

#[derive(thiserror::Error, Debug, Clone)]
pub(crate) enum CallableResolutionError {
    #[error(transparent)]
    SelfResolutionError(#[from] SelfResolutionError),
    #[error(transparent)]
    InputParameterResolutionError(#[from] InputParameterResolutionError),
    #[error(transparent)]
    OutputTypeResolutionError(#[from] OutputTypeResolutionError),
    #[error(transparent)]
    CannotGetCrateData(#[from] CannotGetCrateData),
}

impl From<rustdoc_resolver::CallableResolutionError> for CallableResolutionError {
    fn from(e: rustdoc_resolver::CallableResolutionError) -> Self {
        match e {
            rustdoc_resolver::CallableResolutionError::SelfResolutionError(e) => {
                CallableResolutionError::SelfResolutionError(e)
            }
            rustdoc_resolver::CallableResolutionError::InputParameterResolutionError(e) => {
                CallableResolutionError::InputParameterResolutionError(e)
            }
            rustdoc_resolver::CallableResolutionError::OutputTypeResolutionError(e) => {
                CallableResolutionError::OutputTypeResolutionError(e)
            }
        }
    }
}
