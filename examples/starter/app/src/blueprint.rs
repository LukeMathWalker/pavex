use crate::{routes, telemetry};
use pavex::blueprint::{Blueprint, from};

/// The main blueprint, containing all the routes, middlewares, constructors and error handlers
/// required by our API.
pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    // Bring into scope all macro-annotated components
    // defined in the crates listed via `from!`.
    bp.import(from![
        // Local components, defined in this crate
        crate,
        // Components defined in the `pavex` crate,
        // by the framework itself.
        pavex,
    ]);

    telemetry::register(&mut bp);
    routes::register(&mut bp);
    bp
}
