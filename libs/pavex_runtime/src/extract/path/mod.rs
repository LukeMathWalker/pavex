//! Extract data from the URL of incoming requests using [`PathParams`].
pub use extractor::{
    ExtractPathParamsError, InvalidUtf8InPathParam, PathDeserializationError, PathParams,
};

mod deserializer;
mod extractor;
