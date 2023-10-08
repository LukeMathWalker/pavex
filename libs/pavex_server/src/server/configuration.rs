use std::num::NonZeroUsize;

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
    pub(super) fn new() -> Self {
        let n_workers = match std::thread::available_parallelism() {
            Ok(n) => n,
            Err(e) => {
                let fallback = NonZeroUsize::new(2).unwrap();
                tracing::warn!(
                    error.msg = %e,
                    error.details = ?e,
                    "Failed to determine the amount of available parallelism. \
                    Setting the number of worker threads to a fallback value of {}", fallback);
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
    pub fn n_workers(mut self, n: NonZeroUsize) -> Self {
        self.n_workers = n;
        self
    }
}
