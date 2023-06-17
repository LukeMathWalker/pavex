//! Extract data from the URL of incoming requests.
//!
//! ```rust
//! use pavex::f;
//! use pavex::blueprint::{router::GET, Blueprint, constructor::Lifecycle};
//! use pavex::extract::route::RouteParams;
//!
//! fn blueprint() -> Blueprint{
//!     let mut bp = Blueprint::new();
//!     // [...]
//!     // Register a route with a route parameter, `:home_id`.
//!     bp.route(GET, "/home/:home_id", f!(crate::get_home));
//!     bp
//! }
//!
//! // The RouteParams attribute macro derives the necessary (de)serialization traits.
//! #[RouteParams]
//! struct Home {
//!     // The name of the field must match the name of the route parameter
//!     // used in `bp.route`.
//!     home_id: u32
//! }
//!
//! // The `RouteParams` extractor deserializes the extracted route parameters into
//! // the type you specified—`Home` in this case.
//! fn get_home(params: &RouteParams<Home>) -> String {
//!    format!("The identifier for this home is: {}", params.0.home_id)
//! }
//! ```
//!
//! Check out [`RouteParams`]' documentation for more details.
//!
//! [`RouteParams`]: struct@RouteParams

/// Derive (de)serialization logic for a type that is going to be used to extract route parameters.
///
/// This macro derives [`StructuralDeserialize`], [`serde::Serialize`] and [`serde::Deserialize`]
/// for the type that it is applied to.
///
/// Check out [`RouteParams`](struct@RouteParams) for more details on how to work with
/// route parameters in Pavex.  
/// Check out [`StructuralDeserialize`] if you are curious about the rationale behind this
/// macro.
///
/// # Example
///
/// ```rust
/// use pavex::f;
/// use pavex::blueprint::{router::GET, Blueprint, constructor::Lifecycle};
/// use pavex::extract::route::RouteParams;
///
/// fn blueprint() -> Blueprint { ///
///     let mut bp = Blueprint::new();
///     // [...]
///     // Register a route with a route parameter, `:home_id`.
///     bp.route(GET, "/home/:home_id", f!(crate::get_home));
///     # bp
/// }
///
/// #[RouteParams]
/// struct Home {
///     home_id: u32
/// }
///
/// fn get_home(params: &RouteParams<Home>) -> String {
///     format!("The identifier for this home is: {}", params.0.home_id)
/// }
/// ```
///
/// [`StructuralDeserialize`]: crate::serialization::StructuralDeserialize
pub use pavex_macros::RouteParams;
pub use raw_route_params::{RawRouteParams, RawRouteParamsIter};
pub use route_params::RouteParams;

mod deserializer;
pub mod errors;
mod raw_route_params;
mod route_params;
