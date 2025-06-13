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
/// The method used to create (and set the properties) for this component.
pub enum CreatedBy {
    /// The component was created via a macro annotation (e.g. `#[pavex::wrap]`)
    /// on top of the target item (e.g. a function or a method).
    Attribute { name: String },
    /// The component was created passing a raw path (via `f!` or `t!`) to a
    /// blueprint method.
    Blueprint,
    /// The component was provided by the framework.
    ///
    /// For example, the default fallback handler if the user didn't specify one.
    Framework,
}

impl CreatedBy {
    /// Convert the name of the macro used to perform the registration into an instance of [`CreatedBy`].
    pub fn macro_name(value: &str) -> Self {
        match value {
            "f" | "t" => CreatedBy::Blueprint,
            "pre_process" | "post_process" | "wrap" | "constructor" | "request_scoped"
            | "transient" | "singleton" | "config" | "error_handler" | "error_observer"
            | "fallback" | "route" => CreatedBy::Attribute { name: value.into() },
            _ => panic!(
                "Pavex doesn't recognize `{value}` as one of its macros to register components"
            ),
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnnotationCoordinates {
    /// An opaque string that uniquely identifies this component within the package
    /// where it was defined.
    pub id: String,
    /// Metadata required to pinpoint where the annotated component lives.
    pub created_at: CreatedAt,
    /// The name of the macro used to annotate the component.
    pub macro_name: String,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
/// All the information required to identify a component registered against a `Blueprint`.
///
/// It is an implementation detail of the builder.
pub struct RawIdentifiers {
    /// Information on the location where the component was created—either via `f!`/`t!` using
    /// its import path or via a macro annotation on the definition of the item itself.
    pub created_at: CreatedAt,
    /// The registration system used to create this component.
    pub created_by: CreatedBy,
    /// An unambiguous path to the type/callable.
    pub import_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
/// Information on the crate/module where the component was created.
///
/// This location matches, for example, where the `from!` or the `f!` macro were invoked.
/// For annotated items (e.g. via `#[pavex::config]`), this refers to the location of the annotation.
///
/// It may be different from the location where the component was registered
/// with the blueprint—i.e. where a `Blueprint` method was invoked.
pub struct CreatedAt {
    /// The name of the crate that created the component, as it appears in the `package.name` field
    /// of its `Cargo.toml`.
    /// Obtained via [`CARGO_PKG_NAME`](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates).
    ///
    /// In particular, the name has *not* been normalised—e.g. hyphens are not replaced with underscores.
    ///
    /// This information is needed to resolve the import path unambiguously.
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
    pub package_name: String,
    /// The version of the crate that created the component, as it appears in the `package.version` field
    /// of its `Cargo.toml`.
    ///
    /// Obtained via [`CARGO_PKG_VERSION`](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates).
    pub package_version: String,
    /// The path to the module where the component was created, obtained via [`module_path!`].
    pub module_path: String,
}

impl RawIdentifiers {
    #[track_caller]
    pub fn from_raw_parts(
        import_path: String,
        created_at: CreatedAt,
        created_by: CreatedBy,
    ) -> Self {
        Self {
            created_at,
            created_by,
            import_path,
        }
    }

    /// Return an unambiguous fully-qualified path pointing at the item.
    ///
    /// The returned path should be a valid Rust import path.
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
            segments[0] = self.created_at.package_name.replace('-', "_");
            segments
        } else if segments[0] == "self" {
            // The path is relative to the current module.
            // We "rebase" it to get an absolute path.
            let mut new_segments: Vec<_> = self
                .created_at
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
                let iter = segments.iter();
                for p in iter {
                    if p == "super" {
                        n_super += 1;
                    } else {
                        break;
                    }
                }
                n_super
            };
            // The path is relative to the current module.
            // We "rebase" it to get an absolute path.
            let module_segments: Vec<_> = self
                .created_at
                .module_path
                .split("::")
                .map(|s| s.trim())
                .map(ToOwned::to_owned)
                .collect();
            let n_module_segments = module_segments.len();

            module_segments
                .into_iter()
                .take(n_module_segments - n_super)
                .chain(segments.into_iter().skip(n_super))
                .collect()
        } else {
            segments
        }
    }

    /// The path provided by the user, unaltered.
    pub fn raw_path(&self) -> &str {
        &self.import_path
    }

    pub fn created_at(&self) -> &CreatedAt {
        &self.created_at
    }
}
