#[macro_export]
macro_rules! f {
    ($($p:tt)*) => {{
        #[allow(unused_variables)]
        // First we perform a coarse test to try to ensure that $p is a path to a function
        // or a static method.
        // This is going to raise an error for methods that take self as an argument
        // or for functions with generic parameters that have not been specified explicitly
        // using the turbo-fish syntax.
        let callable = $($p)*;
        stringify!($($p)*)
    }};
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawCallableIdentifiers {
    /// The name of the crate that registered the callable against the blueprint builder.
    /// This information is needed to resolve the callable import path unambiguously.
    ///
    /// E.g. `my_crate::module_1::type_2` - which crate is `my_crate`?
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
    #[track_caller]
    pub fn new(import_path: &'static str) -> Self {
        Self {
            registered_at: std::env::var("CARGO_PKG_NAME").expect("Failed to fetch the CARGO_CRATE_NAME environment variable. Are you using a custom build system?"),
            import_path: import_path.to_string(),
        }
    }

    #[doc(hidden)]
    pub fn from_raw_parts(import_path: String, registered_at: String) -> Self {
        Self {
            registered_at,
            import_path,
        }
    }

    /// Return an unambiguous fully-qualified path pointing at the callable.
    pub fn fully_qualified_path(&self) -> Vec<String> {
        let mut segments: Vec<_> = self
            .import_path
            .split("::")
            .map(|s| s.trim())
            .map(ToOwned::to_owned)
            .collect();
        // Replace the relative portion of the path (`crate`) with the actual crate name.
        if segments[0] == "crate" {
            segments[0] = self.registered_at.clone();
        }
        segments
    }

    /// The path provided by the user, unaltered.
    pub fn raw_path(&self) -> &str {
        &self.import_path
    }

    /// The name of the crate where this callable was registered with a builder.
    pub fn registered_at(&self) -> &str {
        &self.registered_at
    }
}
