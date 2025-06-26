// If a module defines a component (e.g. a route or a middleware or a constructor), it must be
// public. Those components must be importable from the `server_sdk` crate, therefore they must
// be accessible from outside this crate.
pub mod configuration;
pub mod routes;
pub mod telemetry;

use pavex::{Blueprint, blueprint::from};

/// The main blueprint, defining all the components (routes, middlewares, constructors, error handlers, etc.)
/// used in our API.
pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    // Bring into scope constructors, error handlers and configuration
    // types defined in the crates listed via `from!`.
    bp.import(from![
        // Local components, defined in this crate
        crate,
        // Components defined in the `pavex` crate,
        // by the framework itself.
        pavex,
    ]);

    telemetry::instrument(&mut bp);

    // Register all routes defined in this crate,
    // prefixing their paths with `/api`.
    bp.prefix("/api").nest({
        let mut bp = Blueprint::new();
        bp.routes(from![crate]);
        bp
    });
    bp
}
