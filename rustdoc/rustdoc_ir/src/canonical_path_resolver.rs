use rustdoc_ext::GlobalItemId;

/// Resolves a [`GlobalItemId`] to the canonical shortest importable path for that item.
///
/// Used by [`Type::canonicalize`](crate::Type::canonicalize) to rewrite
/// [`PathType::base_type`](crate::PathType::base_type) so that the same underlying type
/// reached via different exports canonicalizes to the same value.
///
/// The trait lives in `rustdoc_ir` to avoid a dependency on `rustdoc_processor`;
/// `rustdoc_processor` provides the concrete implementation on `CrateCollection`.
pub trait CanonicalPathResolver {
    /// Return the canonical shortest importable path for the given item, or `None` if
    /// no canonical path is available.
    fn canonical_path(&self, id: &GlobalItemId) -> Option<Vec<String>>;
}

/// A resolver that never rewrites paths. Intended for tests and for the rare call sites
/// that genuinely have no [`CrateCollection`](https://docs.rs/rustdoc_processor) available.
pub struct NoOpResolver;

impl CanonicalPathResolver for NoOpResolver {
    fn canonical_path(&self, _: &GlobalItemId) -> Option<Vec<String>> {
        None
    }
}
