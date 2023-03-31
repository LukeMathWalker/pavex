use std::borrow::Cow;

use pavex_builder::{constructor::Lifecycle, f, router::GET, Blueprint};
use pavex_runtime::extract::route::RouteParams;

#[RouteParams]
pub struct HomeRouteParams {
    pub home_id: u32,
}

pub fn get_home(RouteParams(HomeRouteParams { home_id }): RouteParams<HomeRouteParams>) -> String {
    format!("{}", home_id)
}

#[RouteParams]
pub struct RoomRouteParams {
    pub home_id: u32,
    // This is not a valid type for a route parameter
    pub room_id: Vec<u32>,
}

pub fn get_room(params: RouteParams<RoomRouteParams>) -> String {
    format!("{}", params.0.home_id)
}

#[RouteParams]
pub struct TownRouteParams<'a> {
    pub town: Cow<'a, str>,
}

pub fn get_town(params: RouteParams<TownRouteParams<'_>>) -> String {
    format!("{}", params.0.town)
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
    bp.route(GET, "/home/:home_id", f!(crate::get_home));
    bp.route(GET, "/home/:home_id/room/:room_id", f!(crate::get_room));
    bp.route(GET, "/town/*town", f!(crate::get_town));
    bp
}
