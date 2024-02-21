use crate::telemetry;
use pavex::blueprint::constructor::CloningStrategy;
use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::kit::ApiKit;

/// The main blueprint, containing all the routes, middlewares, constructors and error handlers
/// required by our API.
pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    ApiKit::new().register(&mut bp);
    telemetry::register(&mut bp);

    bp.route(GET, "/api/ping", f!(crate::routes::status::ping));
    bp
}
