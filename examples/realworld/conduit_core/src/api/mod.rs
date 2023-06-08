use pavex_builder::{Blueprint, constructor::Lifecycle, f, router::GET};

pub mod articles;
pub mod profiles;
pub mod status;
pub mod tags;
pub mod users;

/// The main API blueprint, containing all the routes, constructors and error handlers
/// required to implement the Realworld API specification.
pub fn api_blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    register_common_constructors(&mut bp);
    bp.nest_at("/articles", articles::articles_bp());
    bp.nest_at("/profiles", profiles::profiles_bp());
    bp.nest(users::users_bp());
    bp.route(GET, "/api/ping", f!(crate::api::status::ping));
    bp.route(GET, "/tags", f!(crate::api::tags::get_tags));
    bp
}

/// Common constructors used by all routes.
fn register_common_constructors(bp: &mut Blueprint) {
    // Query parameters
    bp.constructor(
        f!(pavex_runtime::extract::query::QueryParams::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex_runtime::extract::query::errors::ExtractQueryParamsError::into_response
    ));
    
    // Route parameters
    bp.constructor(
        f!(pavex_runtime::extract::route::RouteParams::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex_runtime::extract::route::errors::ExtractRouteParamsError::into_response
    ));

    // Json body
    bp.constructor(
        f!(pavex_runtime::extract::body::JsonBody::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex_runtime::extract::body::errors::ExtractJsonBodyError::into_response
    ));
    bp.constructor(
        f!(pavex_runtime::extract::body::BufferedBody::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex_runtime::extract::body::errors::ExtractBufferedBodyError::into_response
    ));
    bp.constructor(
        f!(<pavex_runtime::extract::body::BodySizeLimit as std::default::Default>::default),
        Lifecycle::RequestScoped,
    );
}