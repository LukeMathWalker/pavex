#[derive(Debug, Hash, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
/// An implementation detail of the builder.
/// You must use the [`f!`] macro wherever a `RawCallable` is needed.
///
/// [`f!`]: crate::f
pub struct RawCallable {
    #[doc(hidden)]
    pub import_path: &'static str,
    #[doc(hidden)]
    pub registered_at: &'static str,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
/// All the information required to identify a callable registered against a [`Blueprint`].
///
/// It is an implementation detail of the builder.
///
/// [`Blueprint`]: crate::blueprint::Blueprint
pub struct RawCallableIdentifiers {
    /// The name of the crate that registered the callable against the blueprint builder.
    /// This information is needed to resolve the callable import path unambiguously.
    ///
    /// E.g. `my_crate::module_1::type_2`—which crate is `my_crate`?
    /// This is not obvious due to the possibility of [renaming dependencies in `Cargo.toml`](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html?highlight=rename,depende#renaming-dependencies-in-cargotoml):
    ///
    /// ```toml
    /// [package]
    /// name = "mypackage"
    /// version = "0.0.1"
    ///
    /// [dependencies]
    /// my_crate = { version = "0.1", registry = "custom", package = "their_crate" }
    /// ```
    registered_at: String,
    /// A fully-qualified path pointing at a callable.
    import_path: String,
}

impl RawCallableIdentifiers {
    #[doc(hidden)]
    #[track_caller]
    pub fn from_raw_parts(import_path: String, registered_at: String) -> Self {
        Self {
            registered_at,
            import_path,
        }
    }

    #[doc(hidden)]
    #[track_caller]
    pub fn from_raw_callable(raw: RawCallable) -> Self {
        Self::from_raw_parts(raw.import_path.to_string(), raw.registered_at.to_string())
    }

    /// Return an unambiguous fully-qualified path pointing at the callable.
    ///
    /// The returned path can be used to import the callable.
    pub fn fully_qualified_path(&self) -> Vec<String> {
        let mut segments: Vec<_> = self
            .import_path
            .split("::")
            .map(|s| s.trim())
            .map(ToOwned::to_owned)
            .collect();
        // Replace the relative portion of the path (`crate`) with the actual crate name.
        if segments[0] == "crate" {
            // Hyphens are allowed in crate names, but the Rust compiler doesn't
            // allow them in actual import paths.
            // They are "transparently" replaced with underscores.
            segments[0] = self.registered_at.replace('-', "_");
        }
        segments
    }

    /// The path provided by the user, unaltered.
    pub fn raw_path(&self) -> &str {
        &self.import_path
    }

    /// The name of the crate where this callable was registered with a builder.
    ///
    /// This is the crate name as it appears in the `package` section of its `Cargo.toml`.
    /// In particular, it has *not* been normalised—e.g. hyphens are not replaced with underscores.
    pub fn registered_at(&self) -> &str {
        &self.registered_at
    }
}
