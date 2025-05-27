use crate::unit::ByteUnit;
use pavex_macros::methods;
use ubyte::ToByteUnit;

#[derive(Debug, Clone, Copy)]
/// An upper limit on the size of incoming request bodies.
///
/// Check out the documentation of [`BufferedBody`](crate::request::body::BufferedBody) for more details.
pub enum BodySizeLimit {
    /// There is an active limit on the size of incoming request bodies.
    Enabled {
        /// The maximum size of incoming request bodies, in bytes.
        max_size: ByteUnit,
    },
    /// There is no limit on the size of incoming request bodies.
    Disabled,
}

#[methods]
impl BodySizeLimit {
    /// Create a new [`BodySizeLimit`] using the default limit (2 MBs).
    #[request_scoped]
    pub fn new() -> BodySizeLimit {
        Self::default()
    }
}

impl Default for BodySizeLimit {
    fn default() -> Self {
        Self::Enabled {
            max_size: 2.megabytes(),
        }
    }
}
