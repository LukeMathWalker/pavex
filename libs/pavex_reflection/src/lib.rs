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
    /// The component was provided by the framework.
    ///
    /// For example, the default fallback handler if the user didn't specify one.
    Framework,
}

impl CreatedBy {
    /// Convert the name of the macro used to perform the registration into an instance of [`CreatedBy`].
    pub fn macro_name(value: &str) -> Self {
        match value {
            "pre_process" | "post_process" | "wrap" | "request_scoped" | "transient"
            | "singleton" | "config" | "error_handler" | "error_observer" | "fallback"
            | "route" => CreatedBy::Attribute { name: value.into() },
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
}
