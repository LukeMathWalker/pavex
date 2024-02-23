#[derive(Clone, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
/// A set of coordinates to identify a precise spot in a source file.
///
/// # Implementation Notes
///
/// `Location` is an owned version of [`std::panic::Location`].
/// You can build a `Location` instance starting from a [`std::panic::Location`]:
///
/// ```rust
/// use pavex_reflection::Location;
///
/// let location: Location = std::panic::Location::caller().into();
/// ```
pub struct Location {
    /// The line number.
    ///
    /// Lines are 1-indexed (i.e. the first line is numbered as 1, not 0).
    pub line: u32,
    /// The column number.
    ///
    /// Columns are 1-indexed (i.e. the first column is numbered as 1, not 0).
    pub column: u32,
    /// The name of the source file.
    ///
    /// Check out [`std::panic::Location::file`] for more details.
    pub file: String,
}

impl<'a> From<&'a std::panic::Location<'a>> for Location {
    fn from(l: &'a std::panic::Location<'a>) -> Self {
        Self {
            line: l.line(),
            column: l.column(),
            file: l.file().into(),
        }
    }
}

impl Location {
    #[track_caller]
    pub fn caller() -> Self {
        std::panic::Location::caller().into()
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
/// All the information required to identify a callable registered against a [`Blueprint`].
///
/// It is an implementation detail of the builder.
///
/// [`Blueprint`]: crate::blueprint::Blueprint
pub struct RawCallableIdentifiers {
    /// Information on the callsite where the callable was registered with the [`Blueprint`].
    pub registered_at: RegisteredAt,
    /// A fully-qualified path pointing at a callable.
    pub import_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
/// Information on the callsite where a callable was registered.
pub struct RegisteredAt {
    /// The name of the crate that registered the callable against the blueprint builder,
    /// as it appears in the `package` section of its `Cargo.toml`.  
    /// In particular,
    /// it has *not* been normalised—e.g. hyphens are not replaced with underscores.  
    ///
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
    pub crate_name: String,
    /// The path to the module where the callable was registered, obtained via [`module_path!`].
    pub module_path: String,
}

impl RawCallableIdentifiers {
    #[track_caller]
    pub fn from_raw_parts(import_path: String, registered_at: RegisteredAt) -> Self {
        Self {
            registered_at,
            import_path,
        }
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
        eprintln!("{:?}", &segments);
        // Replace the relative portion of the path (`crate`) with the actual crate name.
        if segments[0] == "crate" {
            // Hyphens are allowed in crate names, but the Rust compiler doesn't
            // allow them in actual import paths.
            // They are "transparently" replaced with underscores.
            segments[0] = self.registered_at.crate_name.replace('-', "_");
            segments
        } else if segments[0] == "self" {
            // The path is relative to the current module.
            // We "rebase" it to get an absolute path.
            let mut new_segments: Vec<_> = self
                .registered_at
                .module_path
                .split("::")
                .map(|s| s.trim())
                .map(ToOwned::to_owned)
                .collect();
            new_segments.extend(segments.into_iter().skip(1));
            new_segments
        } else if segments[0] == "super" {
            let n_super: usize = {
                let mut n_super = 0;
                let mut iter = segments.iter();
                while let Some(p) = iter.next() {
                    if p == "super" {
                        n_super += 1;
                    } else {
                        break;
                    }
                }
                n_super
            };
            eprintln!("{:?}", n_super);
            // The path is relative to the current module.
            // We "rebase" it to get an absolute path.
            let module_segments: Vec<_> = self
                .registered_at
                .module_path
                .split("::")
                .map(|s| s.trim())
                .map(ToOwned::to_owned)
                .collect();
            let n_module_segments = module_segments.len();
            let new_segments = module_segments
                .into_iter()
                .take(n_module_segments - n_super)
                .chain(segments.into_iter().skip(n_super))
                .collect();
            new_segments
        } else {
            segments
        }
    }

    /// The path provided by the user, unaltered.
    pub fn raw_path(&self) -> &str {
        &self.import_path
    }

    pub fn registered_at(&self) -> &RegisteredAt {
        &self.registered_at
    }
}
