use pavex::blueprint::constructor::CloningStrategy;
use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::kit::ApiKit;

/// The main blueprint, containing all the routes, constructors and error handlers
/// required by our API.
pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    ApiKit::new().register(&mut bp);

    add_telemetry_middleware(&mut bp);

    bp.route(GET, "/api/ping", f!(crate::routes::status::ping));
    bp
}

/// Add the telemetry middleware, as well as the constructors of its dependencies.
fn add_telemetry_middleware(bp: &mut Blueprint) {
    bp.constructor(
        f!(crate::telemetry::RootSpan::new),
        Lifecycle::RequestScoped,
    )
    .cloning(CloningStrategy::CloneIfNecessary);

    bp.wrap(f!(crate::telemetry::logger));
}
