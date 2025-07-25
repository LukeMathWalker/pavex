use crate::telemetry;
use pavex::{Blueprint, blueprint::from};

/// The main blueprint, defining all the components used in this API.
pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    // Bring into scope constructors, error handlers, configuration
    // and prebuilt types defined in the following crates
    bp.import(from![
        // Local components, defined in this crate
        crate,
        // Components defined in the `pavex` crate,
        // by the framework itself.
        pavex,
    ]);

    telemetry::instrument(&mut bp);

    bp.prefix("/api").routes(from![crate]);
    bp
}
