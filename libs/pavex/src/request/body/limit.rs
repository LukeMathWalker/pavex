use crate::blueprint::constructor::{Constructor, Lifecycle, RegisteredConstructor};
use crate::blueprint::Blueprint;
use crate::f;
use crate::unit::ByteUnit;
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

impl BodySizeLimit {
    /// Register the [default constructor](BodySizeLimit::default) for [`BodySizeLimit`] with a [`Blueprint`].
    pub fn register(bp: &mut Blueprint) -> RegisteredConstructor {
        Self::default_constructor().register(bp)
    }

    pub fn default_constructor() -> Constructor {
        Constructor::new(
            f!(<pavex::request::body::BodySizeLimit as std::default::Default>::default),
            Lifecycle::RequestScoped,
        )
    }
}

impl Default for BodySizeLimit {
    fn default() -> Self {
        Self::Enabled {
            max_size: 2.megabytes(),
        }
    }
}
