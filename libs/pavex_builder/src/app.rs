use std::collections::{BTreeMap, HashMap};
use std::fmt::{Display, Formatter};

use indexmap::IndexSet;

use crate::callable::RawCallableIdentifiers;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct AppBlueprint {
    pub constructors: IndexSet<RawCallableIdentifiers>,
    pub handlers: IndexSet<RawCallableIdentifiers>,
    pub component_lifecycles: HashMap<RawCallableIdentifiers, Lifecycle>,
    pub router: BTreeMap<String, RawCallableIdentifiers>,
    pub handler_locations: HashMap<RawCallableIdentifiers, IndexSet<Location>>,
    pub constructor_locations: HashMap<RawCallableIdentifiers, Location>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Lifecycle {
    /// There will be a single instance of the component for each instance of the server.
    ///
    /// As a consequence, the constructor is invoked at most once and the resulting component is
    /// stored as part of the server state. Every time the component is required as input,
    /// the same instance is injected.
    Singleton,
    /// There will be a single instance of the component for every incoming request.
    ///
    /// As a consequence, the constructor is invoked at most once for every incoming request.
    RequestScoped,
    /// The constructor is invoked every single time an instance of the component is required.
    Transient,
}

impl Display for Lifecycle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Lifecycle::Singleton => "singleton",
            Lifecycle::RequestScoped => "request-scoped",
            Lifecycle::Transient => "transient",
        };
        write!(f, "{}", s)
    }
}

impl AppBlueprint {
    pub fn new() -> Self {
        Default::default()
    }

    #[track_caller]
    /// Register a constructor with the application blueprint.
    ///
    /// If a constructor for the same type has already been registered, it will be overwritten.
    pub fn constructor(mut self, import_path: &'static str, lifecycle: Lifecycle) -> Self {
        let callable_identifiers = RawCallableIdentifiers::new(import_path);
        let location = std::panic::Location::caller();
        self.constructor_locations
            .entry(callable_identifiers.clone())
            .or_insert_with(|| location.into());
        self.component_lifecycles
            .insert(callable_identifiers.clone(), lifecycle);
        self.constructors.insert(callable_identifiers);
        self
    }

    #[track_caller]
    /// Register a route and the corresponding request handler with the application blueprint.
    ///
    /// If a handler has already been registered for the same route, it will be overwritten.
    pub fn route(mut self, handler_import_path: &'static str, path: &str) -> Self {
        let callable_identifiers = RawCallableIdentifiers::new(handler_import_path);
        self.handler_locations
            .entry(callable_identifiers.clone())
            .or_default()
            .insert(std::panic::Location::caller().into());
        self.router
            .insert(path.to_owned(), callable_identifiers.clone());
        self.handlers.insert(callable_identifiers);
        self
    }

    /// Serialize the application blueprint as RON and persist it to a file.
    pub fn persist(&self, filepath: &std::path::Path) -> Result<(), anyhow::Error> {
        let mut file = fs_err::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(filepath)?;
        let config = ron::ser::PrettyConfig::new();
        ron::ser::to_writer_pretty(&mut file, &self, config)?;
        Ok(())
    }

    /// Read a RON-serialized application blueprint from a file.
    pub fn load(filepath: &std::path::Path) -> Result<Self, anyhow::Error> {
        let file = fs_err::OpenOptions::new().read(true).open(filepath)?;
        let value = ron::de::from_reader(&file)?;
        Ok(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Location {
    pub line: u32,
    pub column: u32,
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
