use crate::routes;
use pavex::blueprint::constructor::CloningStrategy;
use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::kit::ApiKit;

/// The main API blueprint, containing all the routes, constructors and error handlers
/// required to implement the Realworld API specification.
pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    ApiKit::new().register(&mut bp);
    bp.constructor(
        f!(crate::configuration::DatabaseConfig::get_pool),
        Lifecycle::Singleton,
    );
    bp.constructor(
        f!(crate::configuration::AuthConfig::decoding_key),
        Lifecycle::Singleton,
    );

    setup_telemetry(&mut bp);

    bp.nest_at("/articles", routes::articles::articles_bp());
    bp.nest_at("/profiles", routes::profiles::profiles_bp());
    bp.nest(routes::users::users_bp());
    bp.route(GET, "/api/ping", f!(crate::routes::status::ping));
    bp.route(GET, "/tags", f!(crate::routes::tags::get_tags));
    bp
}

fn setup_telemetry(bp: &mut Blueprint) {
    bp.constructor(f!(crate::telemetry::root_span), Lifecycle::RequestScoped)
        .cloning(CloningStrategy::CloneIfNecessary);

    bp.wrap(f!(pavex_tracing::logger));
    bp.wrap(f!(crate::telemetry::response_logger));
}
