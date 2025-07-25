//! Customize how nested routes should behave.

use pavex_bp_schema::{
    Blueprint as BlueprintSchema, Domain, Location, NestedBlueprint, PathPrefix,
};

use crate::Blueprint;

use super::Import;

/// The type returned by [`Blueprint::prefix`] and [`Blueprint::domain`].
///
/// Customize routing behaviour for a subset of routes.
///
/// [`Blueprint::prefix`]: crate::Blueprint::prefix
/// [`Blueprint::domain`]: crate::Blueprint::domain
#[must_use = "`prefix` and `domain` do nothing unless you invoke `nest` to register some routes under them"]
pub struct RoutingModifiers<'a> {
    pub(super) blueprint: &'a mut BlueprintSchema,
    pub(super) path_prefix: Option<PathPrefix>,
    pub(super) domain: Option<Domain>,
}

impl<'a> RoutingModifiers<'a> {
    pub(super) fn empty(blueprint: &'a mut BlueprintSchema) -> Self {
        Self {
            blueprint,
            path_prefix: None,
            domain: None,
        }
    }

    /// Only requests to the specified domain will be forwarded to routes nested under this condition.
    ///
    /// Check out [`Blueprint::domain`](crate::Blueprint::domain) for more details.
    #[track_caller]
    pub fn domain(mut self, domain: &str) -> Self {
        let location = Location::caller();
        self.domain = Some(Domain {
            domain: domain.into(),
            registered_at: location,
        });
        self
    }

    /// Prepends a common prefix to all routes nested under this condition.
    ///
    /// If a prefix has already been set, it will be overridden.
    ///
    /// Check out [`Blueprint::prefix`](crate::Blueprint::prefix) for more details.
    #[track_caller]
    pub fn prefix(mut self, prefix: &str) -> Self {
        let location = Location::caller();
        self.path_prefix = Some(PathPrefix {
            path_prefix: prefix.into(),
            registered_at: location,
        });
        self
    }

    #[track_caller]
    #[doc(alias("scope"))]
    /// Nest a [`Blueprint`], optionally applying a [common prefix](`Self::prefix`) and a [domain restriction](`Self::domain`) to all its routes.
    ///
    /// Nesting also has consequences when it comes to constructors' visibility.
    ///
    /// # Constructors
    ///
    /// Constructors registered against the parent blueprint will be available to the nested
    /// blueprint—they are **inherited**.
    /// Constructors registered against the nested blueprint will **not** be available to other
    /// sibling blueprints that are nested under the same parent—they are **private**.
    ///
    /// Check out the example below to better understand the implications of nesting blueprints.
    ///
    /// ## Visibility
    ///
    /// ```rust
    /// use pavex::{blueprint::from, Blueprint};
    ///
    /// fn app() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     bp.constructor(DB_CONNECTION_POOL);
    ///     bp.nest(home_bp());
    ///     bp.nest(user_bp());
    ///     bp
    /// }
    ///
    /// /// All property-related routes and constructors.
    /// fn home_bp() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     bp.import(from![crate::home]);
    ///     bp.routes(from![crate::home]);
    ///     bp
    /// }
    ///
    /// /// All user-related routes and constructors.
    /// fn user_bp() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     bp.import(from![crate::user]);
    ///     bp.routes(from![crate::user]);
    ///     bp
    /// }
    ///
    /// # struct ConnectionPool;
    /// #[pavex::singleton]
    /// pub fn db_connection_pool() -> ConnectionPool {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// pub mod home {
    ///     // [...]
    /// }
    ///
    /// pub mod user {
    ///     # struct Session;
    ///     pub fn get_session() -> Session {
    ///         // [...]
    ///         # todo!()
    ///     }
    ///     // [...]
    /// }
    /// ```
    ///
    /// In this example, we import two constructors:
    /// - `crate::user::get_session`, for `Session`;
    /// - `crate::db_connection_pool`, for `ConnectionPool`.
    ///
    /// The constructors defined in the `crate::user` module are only imported by the `user_bp` blueprint.
    /// Since we are **nesting** the `user_bp` blueprint, those constructors will only be available
    /// to the routes declared in the `user_bp` blueprint.
    /// If a route declared in `home_bp` tries to inject a `Session`, Pavex will report an error
    /// at compile-time, complaining that there is no registered constructor for `Session`.
    /// In other words, all constructors imported in the `user_bp` blueprint are **private**
    /// and **isolated** from the rest of the application.
    ///
    /// The `db_connection_pool` constructor, instead, is declared against the parent blueprint
    /// and will therefore be available to all routes declared in `home_bp` and `user_bp`—i.e.
    /// nested blueprints **inherit** all the constructors declared against their parent(s).
    ///
    /// ## Precedence
    ///
    /// If a constructor is declared against both the parent and one of its nested blueprints, the one
    /// declared against the nested blueprint takes precedence.
    ///
    /// ```rust
    /// use pavex::{blueprint::from, Blueprint};
    ///
    /// fn app() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     // These constructors are registered against the root blueprint and they're visible
    ///     // to all nested blueprints.
    ///     bp.import(from![crate::global]);
    ///     bp.nest(user_bp());
    ///     // [..]
    ///     bp
    /// }
    ///
    /// fn user_bp() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     // They can be overridden by a constructor for the same type registered
    ///     // against a nested blueprint.
    ///     // All routes in `user_bp` will use `user::get_session` instead of `global::get_session`.
    ///     bp.import(from![crate::user]);
    ///     // [...]
    ///     bp
    /// }
    ///
    /// pub mod global {
    ///     # struct Session;
    ///     pub fn get_session() -> Session {
    ///         // [...]
    ///         # todo!()
    ///     }
    /// }
    ///
    /// pub mod user {
    ///     # struct Session;
    ///     pub fn get_session() -> Session {
    ///         // [...]
    ///         # todo!()
    ///     }
    /// }
    /// ```
    ///
    /// ## Singletons
    ///
    /// There is one exception to the precedence rule: [singletons][Lifecycle::Singleton].
    /// Pavex guarantees that there will be only one instance of a singleton type for the entire
    /// lifecycle of the application. What should happen if two different constructors are registered for
    /// the same `Singleton` type by two nested blueprints that share the same parent?
    /// We can't honor both constructors without ending up with two different instances of the same
    /// type, which would violate the singleton contract.
    ///
    /// It goes one step further! Even if those two constructors are identical, what is the expected
    /// behaviour? Does the user expect the same singleton instance to be injected in both blueprints?
    /// Or does the user expect two different singleton instances to be injected in each nested blueprint?
    ///
    /// To avoid this ambiguity, Pavex takes a conservative approach: a singleton constructor
    /// must be registered **exactly once** for each type.
    /// If multiple nested blueprints need access to the singleton, the constructor must be
    /// registered against a common parent blueprint—the root blueprint, if necessary.
    ///
    /// [Lifecycle::Singleton]: crate::blueprint::Lifecycle::Singleton
    pub fn nest(self, bp: Blueprint) {
        self.blueprint.components.push(
            NestedBlueprint {
                blueprint: bp.schema,
                path_prefix: self.path_prefix,
                nested_at: Location::caller(),
                domain: self.domain,
            }
            .into(),
        );
    }

    #[track_caller]
    /// Register a group of routes.
    ///
    /// Their path will be prepended with a common prefix if one was provided via [`.prefix()`][`Self::prefix`].
    /// They will be restricted to a specific domain if one was specified via [`.domain()`][`Self::domain`].
    ///
    /// # Example
    ///
    /// ```
    /// use pavex::{Blueprint, blueprint::from};
    ///
    /// let mut bp = Blueprint::new();
    /// bp.prefix("/api").routes(from![crate::api]);
    /// ```
    pub fn routes(self, import: Import) {
        let mut bp = Blueprint::new();
        bp.routes(import);
        self.nest(bp);
    }
}
