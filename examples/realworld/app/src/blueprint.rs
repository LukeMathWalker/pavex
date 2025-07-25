use crate::routes::router;
use crate::telemetry;
use pavex::cookie::INJECT_RESPONSE_COOKIES;
use pavex::{Blueprint, blueprint::from};

/// The main API blueprint, containing all the routes, constructors and error handlers
/// required to implement the Realworld API specification.
pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate, pavex]);

    // Middleware stack
    telemetry::instrument(&mut bp);
    bp.post_process(INJECT_RESPONSE_COOKIES);

    router(&mut bp);
    bp
}
