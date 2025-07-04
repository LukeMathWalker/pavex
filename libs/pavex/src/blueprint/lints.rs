#[derive(Debug, Clone, Copy, Eq, Ord, PartialOrd, PartialEq, Hash)]
#[non_exhaustive]
/// Common mistakes and antipatterns that Pavex
/// tries to catch when analysing your [`Blueprint`].
///
/// These issues aren't considered fatal: Pavex will still
/// generate the server SDK code.
///
/// [`Blueprint`]: crate::Blueprint
pub enum Lint {
    /// You registered a component that's never used in the generated
    /// server SDK code.
    Unused,
}
