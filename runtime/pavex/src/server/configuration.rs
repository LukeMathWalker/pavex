use std::num::NonZeroUsize;

use tracing_log_error::log_error;

#[derive(Debug, Clone)]
/// All the available options for customizing the behaviour of a [`Server`](super::Server).
///
/// Refer to [`Server::set_config`](super::Server::set_config) for applying the configuration
/// you assembled.
pub struct ServerConfiguration {
    /// Number of worker threads to spawn.
    pub(crate) n_workers: NonZeroUsize,
}

impl Default for ServerConfiguration {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerConfiguration {
    /// Initialize a new [`ServerConfiguration`] using its default settings.
    pub fn new() -> Self {
        let n_workers = match std::thread::available_parallelism() {
            Ok(n) => n,
            Err(e) => {
                let fallback = NonZeroUsize::new(2).unwrap();
                log_error!(
                    e,
                    level: tracing::Level::WARN,
                    "Failed to determine the amount of available parallelism. \
                    Setting the number of worker threads to a fallback value of {}",
                    fallback);
                fallback
            }
        };
        Self { n_workers }
    }

    /// Set the number of worker threads to be spawned.
    /// It must be greater than 0.
    ///
    /// # Default
    ///
    /// It relies on [`std::thread::available_parallelism`] to determine the available parallelism.
    /// On most platforms, this is the number of physical CPU cores available. If the available
    /// parallelism cannot be determined, it defaults to 2.
    ///
    /// ## Logical vs physical Cores
    ///
    /// If you'd prefer to match the number of _logical_ cores instead, you can use the [`num_cpus`]
    /// crate to acquire the logical core count instead.
    ///
    /// [`num_cpus`]: https://docs.rs/num_cpus
    #[track_caller]
    pub fn set_n_workers(mut self, n: usize) -> Self {
        assert!(n > 0, "The number of workers must be greater than 0");
        self.n_workers = NonZeroUsize::new(n).unwrap();
        self
    }

    /// Get the number of worker threads to be spawned.
    pub fn get_n_workers(&self) -> NonZeroUsize {
        self.n_workers
    }
}
