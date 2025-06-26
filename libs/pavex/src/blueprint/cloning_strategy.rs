#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
/// Determines whether Pavex is allowed to clone a type.
///
/// This applies to [configuration types](crate::Blueprint::config),
/// [prebuilt types](crate::Blueprint::prebuilt) and [constructors](crate::Blueprint::constructor).
///
/// # Guide
///
/// Check out the ["Dependency injection"](https://pavex.dev/docs/guide/dependency_injection)
/// section of Pavex's guide for a thorough introduction to dependency injection
/// in Pavex applications.
pub enum CloningStrategy {
    /// Pavex is not allowed to clone the type.\
    /// Pavex will return an error if cloning is necessary to generate code
    /// that satisfies Rust's borrow checker.
    NeverClone,
    /// Pavex is allowed to clone the type.\
    /// Pavex will invoke `.clone()` if it's necessary to generate code that
    /// satisfies Rust's borrow checker.
    CloneIfNecessary,
}
