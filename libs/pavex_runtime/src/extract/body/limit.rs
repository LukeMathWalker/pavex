#[derive(Debug, Clone, Copy)]
/// An upper limit on the size of incoming request bodies.  
///
/// Check out the documentation of [`BufferedBody`](crate::extract::body::BufferedBody) for more details.
pub enum BodySizeLimit {
    /// There is an active limit on the size of incoming request bodies.
    Enabled {
        /// The maximum size of incoming request bodies, in bytes.
        max_n_bytes: usize,
    },
    /// There is no limit on the size of incoming request bodies.
    Disabled,
}

impl Default for BodySizeLimit {
    fn default() -> Self {
        Self::Enabled {
            max_n_bytes: 2_097_152, // 2 MBs
        }
    }
}
