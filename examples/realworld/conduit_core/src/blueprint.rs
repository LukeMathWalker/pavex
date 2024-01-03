use crate::routes;
use pavex::blueprint::constructor::CloningStrategy;
use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;

/// The main API blueprint, containing all the routes, constructors and error handlers
/// required to implement the Realworld API specification.
pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    register_common_constructors(&mut bp);
    bp.constructor(
        f!(crate::configuration::DatabaseConfig::get_pool),
        Lifecycle::Singleton,
    );
    bp.constructor(
        f!(crate::configuration::AuthConfig::decoding_key),
        Lifecycle::Singleton,
    );

    add_telemetry_middleware(&mut bp);

    bp.nest_at("/articles", routes::articles::articles_bp());
    bp.nest_at("/profiles", routes::profiles::profiles_bp());
    bp.nest(routes::users::users_bp());
    bp.route(GET, "/api/ping", f!(crate::routes::status::ping));
    bp.route(GET, "/tags", f!(crate::routes::tags::get_tags));
    bp
}

/// Common constructors used by all routes.
fn register_common_constructors(bp: &mut Blueprint) {
    // Query parameters
    bp.constructor(
        f!(pavex::request::query::QueryParams::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex::request::query::errors::ExtractQueryParamsError::into_response
    ));

    // Route parameters
    bp.constructor(
        f!(pavex::request::path::PathParams::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex::request::path::errors::ExtractPathParamsError::into_response
    ));

    // Json body
    bp.constructor(
        f!(pavex::request::body::JsonBody::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex::request::body::errors::ExtractJsonBodyError::into_response
    ));
    bp.constructor(
        f!(pavex::request::body::BufferedBody::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex::request::body::errors::ExtractBufferedBodyError::into_response
    ));
    bp.constructor(
        f!(<pavex::request::body::BodySizeLimit as std::default::Default>::default),
        Lifecycle::RequestScoped,
    );
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
