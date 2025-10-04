//! Configuration for the CLI client.

/// Control whether to use colors in the CLI output.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Color {
    /// Automatically determine whether to use colors.
    Auto,
    /// Always use colors.
    Always,
    /// Never use colors.
    Never,
}
