#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
/// Determines whether Pavex is allowed to clone a component—e.g. the output type returned by a constructor
/// or a state input.
pub enum CloningStrategy {
    /// Pavex will **never** try clone the type.  
    /// Pavex will return an error if cloning is necessary to generate code
    /// that satisfies Rust's borrow checker.
    NeverClone,
    /// Pavex will only clone the type if it's
    /// necessary to generate code that satisfies Rust's borrow checker.
    CloneIfNecessary,
}
