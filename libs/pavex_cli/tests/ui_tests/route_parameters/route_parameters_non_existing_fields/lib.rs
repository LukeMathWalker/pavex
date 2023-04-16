use pavex_builder::{constructor::Lifecycle, f, router::GET, Blueprint};
use pavex_runtime::extract::route::RouteParams;

#[RouteParams]
pub struct MissingOne {
    x: u32,
    y: u32,
}

pub fn missing_one(params: RouteParams<MissingOne>) -> String {
    todo!()
}

#[RouteParams]
pub struct MissingTwo {
    x: u32,
    y: u32,
    z: u32,
}

pub fn missing_two(params: RouteParams<MissingTwo>) -> String {
    todo!()
}

#[RouteParams]
pub struct NoRouteParams {
    x: u32,
    y: u32,
}

pub fn no_route_params(params: RouteParams<NoRouteParams>) -> String {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(
        f!(pavex_runtime::extract::route::RouteParams::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex_runtime::extract::route::errors::ExtractRouteParamsError::into_response
    ));
    bp.route(GET, "/a/:x", f!(crate::missing_one));
    bp.route(GET, "/b/:x", f!(crate::missing_two));
    bp.route(GET, "/c", f!(crate::no_route_params));
    bp
}
