use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::{extract::route::RouteParams, http::StatusCode};

#[RouteParams]
pub struct MissingOne {
    x: u32,
    y: u32,
}

pub fn missing_one(params: RouteParams<MissingOne>) -> StatusCode {
    todo!()
}

#[RouteParams]
pub struct MissingTwo {
    x: u32,
    y: u32,
    z: u32,
}

pub fn missing_two(params: RouteParams<MissingTwo>) -> StatusCode {
    todo!()
}

#[RouteParams]
pub struct NoRouteParams {
    x: u32,
    y: u32,
}

pub fn no_route_params(params: RouteParams<NoRouteParams>) -> StatusCode {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(
        f!(pavex::extract::route::RouteParams::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex::extract::route::errors::ExtractRouteParamsError::into_response
    ));
    bp.route(GET, "/a/:x", f!(crate::missing_one));
    bp.route(GET, "/b/:x", f!(crate::missing_two));
    bp.route(GET, "/c", f!(crate::no_route_params));
    bp
}
