use std::panic::Location;

use crate::blueprint::{
    Blueprint,
    conversions::{created_at2created_at, sources2sources},
    reflection::{Sources, WithLocation},
};

use super::RegisteredImport;

/// An import that has been configured but has not yet been registered with a [`Blueprint`].
///
/// # Guide
///
/// Check out the ["Dependency injection"](https://pavex.dev/docs/guide/dependency_injection)
/// section of Pavex's guide for a thorough introduction to dependency injection
/// in Pavex applications.
#[derive(Clone, Debug)]
pub struct Import {
    pub(in crate::blueprint) import: pavex_bp_schema::Import,
}

impl Import {
    /// Create a new (unregistered) import.
    ///
    /// Check out the documentation of [`Blueprint::import`] for more details
    /// on imports.
    #[track_caller]
    pub fn new(sources: WithLocation<Sources>) -> Self {
        let WithLocation {
            created_at,
            value: sources,
        } = sources;
        Self {
            import: pavex_bp_schema::Import {
                sources: sources2sources(sources),
                created_at: created_at2created_at(created_at),
                registered_at: Location::caller().into(),
            },
        }
    }

    /// Register this import with a [`Blueprint`].
    ///
    /// Check out the documentation of [`Blueprint::import`] for more details.
    pub fn register(self, bp: &mut Blueprint) -> RegisteredImport {
        bp.register_import(self.import)
    }
}
