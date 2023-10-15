use std::time::Duration;

#[derive(Debug, Clone)]
#[non_exhaustive]
/// Determine how a running [`Server`](super::Server) should shut down.
///
/// Use [`ServerHandle::shutdown`](super::ServerHandle::shutdown) to initiate the shutdown sequence.
pub enum ShutdownMode {
    /// Wait for each worker thread to finish handling its open connections before shutting down.
    Graceful {
        /// How long to wait for the worker to finish handling its open connections.  
        /// Any connection that has not been handled within the timeout will be dropped.
        timeout: Duration,
    },
    /// Shut down immediately, dropping all open connections abruptly.
    Forced,
}

impl ShutdownMode {
    /// Returns `true` if you are asking for a graceful shutdown.
    pub fn is_graceful(&self) -> bool {
        matches!(self, Self::Graceful { .. })
    }

    /// Returns `true` if you are asking for a forced shutdown.
    pub fn is_forced(&self) -> bool {
        matches!(self, Self::Forced)
    }
}
