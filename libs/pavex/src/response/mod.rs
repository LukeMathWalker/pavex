//! Build HTTP responses, from scratch or by converting existing types.
//!
//! Check out the [`Response`] type for more details.
pub use into_response::IntoResponse;

pub mod body;
mod into_response;
mod response_;

pub use response_::{Response, ResponseHead};
