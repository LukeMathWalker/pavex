//! All environment variables that `pavex_cli` reads, apart from the ones
//! defined in the CLI args themselves, are defined here.
//!
//! The goal: avoid random `std::env::var` calls all over the place,
//! and instead have a single place where all environment variables are
//! defined and documented.

use std::path::PathBuf;

/// Set a specific `pavexc` binary to be used, regardless of the
/// logic that would otherwise be used to determine it.
pub fn pavexc_override() -> Option<PathBuf> {
    std::env::var("PAVEX_PAVEXC").ok().map(PathBuf::from)
}

/// This is an undocumented feature that allows us to force set the width of the
/// terminal as seen by the graphical error handler.
/// This is useful for testing/doc-generation purposes.
pub fn tty_width() -> Option<usize> {
    std::env::var("PAVEX_TTY_WIDTH")
        .ok()
        .and_then(|s| s.parse().ok())
}

/// The SHA of the commit that `pavex_cli` was built from.
pub const fn commit_sha() -> &'static str {
    env!("VERGEN_GIT_SHA")
}

/// The version of `pavex_cli` that is being used.
pub fn version() -> semver::Version {
    semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .expect("CLI version is not valid according to SemVer")
}
