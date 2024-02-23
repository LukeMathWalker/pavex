use crate::configuration::ApplicationConfig;
use crate::routes::articles::articles_bp;
use crate::routes::profiles::profiles_bp;
use crate::routes::users::users_bp;
use crate::telemetry;
use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::kit::ApiKit;

/// The main API blueprint, containing all the routes, constructors and error handlers
/// required to implement the Realworld API specification.
pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    ApiKit::new().register(&mut bp);
    telemetry::register(&mut bp);
    ApplicationConfig::register(&mut bp);

    bp.nest_at("/articles", articles_bp());
    bp.nest_at("/profiles", profiles_bp());
    bp.nest(users_bp());
    bp.route(GET, "/api/ping", f!(crate::routes::status::ping));
    bp.route(GET, "/tags", f!(crate::routes::tags::get_tags));
    bp
}
