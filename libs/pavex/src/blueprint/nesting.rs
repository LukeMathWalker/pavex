//! Customize how nested routes should behave.

use pavex_bp_schema::{
    Blueprint as BlueprintSchema, Domain, Location, NestedBlueprint, PathPrefix,
};

use super::Blueprint;

/// The type returned by [`Blueprint::prefix`] and [`Blueprint::domain`].
///
/// It allows you to customize how nested routes should behave.
///
/// [`Blueprint::prefix`]: crate::blueprint::Blueprint::prefix
/// [`Blueprint::domain`]: crate::blueprint::Blueprint::domain
#[must_use = "`prefix` and `domain` do nothing unless you invoke `nest` to register some routes under them"]
pub struct NestingConditions<'a> {
    pub(super) blueprint: &'a mut BlueprintSchema,
    pub(super) path_prefix: Option<PathPrefix>,
    pub(super) domain: Option<Domain>,
}

impl<'a> NestingConditions<'a> {
    pub(super) fn empty(blueprint: &'a mut BlueprintSchema) -> Self {
        Self {
            blueprint,
            path_prefix: None,
            domain: None,
        }
    }

    /// Only requests to the specified domain will be forwarded to routes nested under this condition.
    ///
    /// Check out [`Blueprint::domain`](crate::blueprint::Blueprint::domain) for more details.
    #[track_caller]
    pub fn domain(mut self, domain: &str) -> Self {
        let location = Location::caller();
        self.domain = Some(Domain {
            domain: domain.into(),
            location,
        });
        self
    }

    /// Prepends a common prefix to all routes nested under this condition.
    ///
    /// If a prefix has already been set, it will be overridden.
    ///
    /// Check out [`Blueprint::prefix`](crate::blueprint::Blueprint::prefix) for more details.
    #[track_caller]
    pub fn prefix(mut self, prefix: &str) -> Self {
        let location = Location::caller();
        self.path_prefix = Some(PathPrefix {
            path_prefix: prefix.into(),
            location,
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
    /// use pavex::f;
    /// use pavex::blueprint::{Blueprint, router::GET};
    ///
    /// fn app() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     bp.singleton(f!(crate::db_connection_pool));
    ///     bp.nest(home_bp());
    ///     bp.nest(user_bp());
    ///     bp
    /// }
    ///
    /// /// All property-related routes and constructors.
    /// fn home_bp() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     bp.route(GET, "/home", f!(crate::v1::get_home));
    ///     bp
    /// }
    ///
    /// /// All user-related routes and constructors.
    /// fn user_bp() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     bp.request_scoped(f!(crate::user::get_session));
    ///     bp.route(GET, "/user", f!(crate::user::get_user));
    ///     bp
    /// }
    /// # pub fn db_connection_pool() {}
    /// # mod home { pub fn get_home() {} }
    /// # mod user {
    /// #     pub fn get_user() {}
    /// #     pub fn get_session() {}
    /// # }
    /// ```
    ///
    /// This example registers two routes:
    /// - `GET /home`
    /// - `GET /user`
    ///
    /// It also registers two constructors:
    /// - `crate::user::get_session`, for `Session`;
    /// - `crate::db_connection_pool`, for `ConnectionPool`.
    ///
    /// Since we are **nesting** the `user_bp` blueprint, the `get_session` constructor will only
    /// be available to the routes declared in the `user_bp` blueprint.
    /// If a route declared in `home_bp` tries to inject a `Session`, Pavex will report an error
    /// at compile-time, complaining that there is no registered constructor for `Session`.
    /// In other words, all constructors declared against the `user_bp` blueprint are **private**
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
    /// use pavex::f;
    /// use pavex::blueprint::{Blueprint, router::GET};
    ///
    /// fn app() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     // This constructor is registered against the root blueprint and it's visible
    ///     // to all nested blueprints.
    ///     bp.request_scoped(f!(crate::global::get_session));
    ///     bp.nest(user_bp());
    ///     // [..]
    ///     bp
    /// }
    ///
    /// fn user_bp() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     // It can be overridden by a constructor for the same type registered
    ///     // against a nested blueprint.
    ///     // All routes in `user_bp` will use `user::get_session` instead of `global::get_session`.
    ///     bp.request_scoped(f!(crate::user::get_session));
    ///     // [...]
    ///     bp
    /// }
    /// # mod global { pub fn get_session() {} }
    /// # mod user {
    /// #     pub fn get_user() {}
    /// #     pub fn get_session() {}
    /// # }
    /// ```
    ///
    /// ## Singletons
    ///
    /// There is one exception to the precedence rule: [singletons](`Blueprint::singleton`).
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
    pub fn nest(self, bp: Blueprint) {
        self.blueprint.components.push(
            NestedBlueprint {
                blueprint: bp.schema,
                path_prefix: self.path_prefix,
                nesting_location: Location::caller(),
                domain: self.domain,
            }
            .into(),
        );
    }
}