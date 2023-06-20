use crate::routes;
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
        f!(pavex::extract::query::QueryParams::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex::extract::query::errors::ExtractQueryParamsError::into_response
    ));

    // Route parameters
    bp.constructor(
        f!(pavex::extract::route::RouteParams::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex::extract::route::errors::ExtractRouteParamsError::into_response
    ));

    // Json body
    bp.constructor(
        f!(pavex::extract::body::JsonBody::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex::extract::body::errors::ExtractJsonBodyError::into_response
    ));
    bp.constructor(
        f!(pavex::extract::body::BufferedBody::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex::extract::body::errors::ExtractBufferedBodyError::into_response
    ));
    bp.constructor(
        f!(<pavex::extract::body::BodySizeLimit as std::default::Default>::default),
        Lifecycle::RequestScoped,
    );
}
