use articles::articles_bp;
use pavex_builder::{constructor::Lifecycle, f, router::GET, Blueprint};
use pavex_runtime::hyper::StatusCode;

pub mod articles;

pub fn ping() -> StatusCode {
    StatusCode::OK
}

pub fn api_blueprint() -> Blueprint {
    let mut bp = Blueprint::new();

    bp.constructor(
        f!(pavex_runtime::extract::query::QueryParams::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex_runtime::extract::query::errors::ExtractQueryParamsError::into_response
    ));
    bp.constructor(
        f!(pavex_runtime::extract::route::RouteParams::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex_runtime::extract::route::errors::ExtractRouteParamsError::into_response
    ));
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

    bp.route(GET, "/api/ping", f!(crate::ping));
    bp.nest_at("/articles", articles_bp());
    bp
}
