//! Process and extract data from incoming HTTP requests.
pub use request_head::RequestHead;

pub mod body;
pub mod query;
mod request_head;
pub mod route;
