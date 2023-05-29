//! Serialization and deserialization utilities.

/// A marker trait for types that perform deserialization using the strategy provided "out-of-the-box" by `serde`.
///
/// All types that derive `RouteParams` automatically implement this trait.  
/// It is **discouraged** to manually implement this trait for one of your types—and you should
/// have no need to do so.
///
/// # Why do we need this?
///
/// > This is largely an implementation detail of Pavex and you don't need to worry about it
/// > unless you are curious and want to know more about how Pavex works under the hood.
///
/// This trait is used by Pavex to reason about the way a type is going to be deserialized—i.e.
/// mapping the shape of the Rust type to the expected shape of the data to be deserialized.
///
/// This enables Pavex to confidently detect common errors at compile time—e.g. if a type
/// is trying to deserialize a route parameter that doesn't exist in the route template for the
/// relevant request handler.  
/// Doing this analysis for arbitrary types would result in false positives—e.g. a type might resort to
/// a custom implementation of `serde::Deserialize` that does not actually look for a route parameter
/// named as the field that we see in the type definition.  
/// `StructuralDeserialize` acts as a tag that tells Pavex that a type should be in scope
/// for additional static analysis and that it's OK to make certain assumptions.
pub trait StructuralDeserialize {}
