//! Extract data from the URL of incoming requests using [`RouteParams`].
//!
//! Check out [`RouteParams`]' documentation for more details.
pub use matchit::Params as RawRouteParams;

pub use extractor::{
    ExtractRouteParamsError, InvalidUtf8InPathParam, PathDeserializationError, RouteParams,
};

mod deserializer;
mod extractor;
