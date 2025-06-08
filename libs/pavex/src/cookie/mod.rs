//! Everything you need to work with HTTP cookies.
//!
//! # Guide
//!
//! Check out the ["Cookies"](https://pavex.dev/guide/cookies/)
//! section of Pavex's guide for a thorough introduction to cookies.
//!
//! # Implementation details
//!
//! Most types and functions are re-exports of the
//! [`biscotti@0.3`](https://docs.rs/biscotti/0.3) crate.
// Everything from `biscotti`, except:
// - the `time` module, which is re-exported as a top-level module in Pavex itself
// - `ResponseCookies`, which is customized in the `response_cookies` module
// - the `errors` module, which is augmented with additional error types in the `errors` module
// - the `response` module, which is replaced with a wrapped version in the `response` module
#[crate::config(key = "cookies")]
pub use biscotti::ProcessorConfig;
pub use biscotti::{
    Expiration, Key, Processor, RemovalCookie, RequestCookie, RequestCookies, ResponseCookie,
    ResponseCookieId, SameSite, config, request,
};
pub mod errors;
pub mod response;

mod components;
pub use components::{extract_request_cookies, inject_response_cookies};

mod kit;
pub use kit::CookieKit;

mod response_cookies;
pub use response_cookies::ResponseCookies;
