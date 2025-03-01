use crate::configuration;
use crate::routes::articles::articles_bp;
use crate::routes::profiles::profiles_bp;
use crate::routes::users::users_bp;
use crate::telemetry;
use pavex::blueprint::{Blueprint, router::GET};
use pavex::cookie::CookieKit;
use pavex::f;
use pavex::kit::ApiKit;

/// The main API blueprint, containing all the routes, constructors and error handlers
/// required to implement the Realworld API specification.
pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    ApiKit::new().register(&mut bp);
    telemetry::register(&mut bp);
    configuration::register(&mut bp);
    CookieKit::new().register(&mut bp);

    bp.prefix("/articles").nest(articles_bp());
    bp.prefix("/profiles").nest(profiles_bp());
    bp.nest(users_bp());
    bp.route(GET, "/api/ping", f!(crate::routes::status::ping));
    bp.route(GET, "/tags", f!(crate::routes::tags::get_tags));
    bp
}
