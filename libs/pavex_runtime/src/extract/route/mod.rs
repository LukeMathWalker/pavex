//! Extract data from the URL of incoming requests.
//!
//! ```rust
//! use pavex_builder::{f, router::GET, Blueprint, Lifecycle};
//! use pavex_runtime::extract::route::RouteParams;
//!
//! # fn main() {
//! let mut bp = Blueprint::new();
//! // [...]
//! // Register a route with a route parameter, `:home_id`.
//! bp.route(GET, "/home/:home_id", f!(crate::get_home));
//! # }
//!
//! #[derive(serde::Deserialize)]
//! struct HomeRouteParams {
//!     // The name of the field must match the name of the route parameter
//!     // used in `bp.route`.
//!     home_id: u32
//! }
//!
//! // The `RouteParams` extractor will deserialize the route parameters into
//! // the type you specified—`HomeRouteParams` in this case.
//! fn get_home(params: &RouteParams<HomeRouteParams>) -> String {
//!    format!("The identifier for this home is: {}", params.0.home_id)
//! }
//! ```
//!
//! Check out [`RouteParams`]' documentation for more details.
pub use extractor::{RawRouteParams, RouteParams};

mod deserializer;
pub mod errors;
mod extractor;
