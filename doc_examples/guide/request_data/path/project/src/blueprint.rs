use pavex::blueprint::constructor::Lifecycle;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    register_common_constructors(&mut bp);
    bp.nest(crate::path::blueprint());
    bp.nest(crate::route_params::blueprint());
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
        f!(pavex::request::route::RouteParams::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex::request::route::errors::ExtractRouteParamsError::into_response
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
