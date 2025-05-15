//! Extract data from the URL of incoming requests.
//!
//! # Overview
//!
//! When it comes to route information, there are two important extractors to be aware of:
//!
//! - [`PathParams`]: extract path parameters from the URL of incoming requests
//! - [`MatchedPathPattern`]: extract the route template that matched for the incoming request
//!
//! Check out their documentation for more details.
//!
//! # Example: path parameters
//!
//! ```rust
//! use pavex::f;
//! use pavex::blueprint::{router::GET, Blueprint, constructor::Lifecycle};
//! use pavex::request::path::PathParams;
//!
//! fn blueprint() -> Blueprint{
//!     let mut bp = Blueprint::new();
//!     // [...]
//!     // Register a route with a path parameter, `:home_id`.
//!     bp.route(GET, "/home/:home_id", f!(crate::get_home));
//!     bp
//! }
//!
//! // The PathParams attribute macro derives the necessary (de)serialization traits.
//! #[PathParams]
//! struct Home {
//!     // The name of the field must match the name of the path parameter
//!     // used in `bp.route`.
//!     home_id: u32
//! }
//!
//! // The `PathParams` extractor deserializes the extracted path parameters into
//! // the type you specified—`Home` in this case.
//! fn get_home(params: &PathParams<Home>) -> String {
//!    format!("The identifier for this home is: {}", params.0.home_id)
//! }
//! ```
//!
//! Check out [`PathParams`]' documentation for more details.
//!
//! [`PathParams`]: struct@PathParams

pub use matched_path::MatchedPathPattern;
pub use path_params::PathParams;
/// Derive (de)serialization logic for a type that is going to be used to extract path parameters.
///
/// This macro derives [`StructuralDeserialize`], [`serde::Serialize`] and [`serde::Deserialize`]
/// for the type that it is applied to.
///
/// Check out [`PathParams`](struct@PathParams) for more details on how to work with
/// path parameters in Pavex.
/// Check out [`StructuralDeserialize`] if you are curious about the rationale behind this
/// macro.
///
/// # Example
///
/// ```rust
/// use pavex::f;
/// use pavex::blueprint::{router::GET, Blueprint, constructor::Lifecycle};
/// use pavex::request::path::PathParams;
///
/// fn blueprint() -> Blueprint { ///
///     let mut bp = Blueprint::new();
///     // [...]
///     // Register a route with a path parameter, `:home_id`.
///     bp.route(GET, "/home/:home_id", f!(crate::get_home));
///     # bp
/// }
///
/// #[PathParams]
/// struct Home {
///     home_id: u32
/// }
///
/// fn get_home(params: &PathParams<Home>) -> String {
///     format!("The identifier for this home is: {}", params.0.home_id)
/// }
/// ```
///
/// [`StructuralDeserialize`]: crate::serialization::StructuralDeserialize
pub use pavex_macros::PathParams;
pub use raw_path_params::{EncodedParamValue, RawPathParams, RawPathParamsIter};

mod deserializer;
pub mod errors;

mod matched_path;
mod path_params;
mod raw_path_params;
