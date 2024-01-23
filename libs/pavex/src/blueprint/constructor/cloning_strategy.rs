#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
/// Determines whether Pavex is allowed to clone the output type returned by a constructor.
///
/// Check out [`Constructor::cloning`] for more information.
pub enum CloningStrategy {
    /// Pavex will **never** try clone the output type returned by the constructor.
    NeverClone,
    /// Pavex will only clone the output type returned by this constructor if it's
    /// necessary to generate code that satisfies Rust's borrow checker.
    CloneIfNecessary,
}
