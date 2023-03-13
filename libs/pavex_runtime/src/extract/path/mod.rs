//! Extract the values of templated path segments from the incoming request using [`PathParams`].
pub use extractor::{
    ExtractPathParamsError, InvalidUtf8InPathParam, PathDeserializationError, PathParams,
};

mod deserializer;
mod extractor;
