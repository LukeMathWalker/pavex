use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::{request::path::PathParams, response::Response};

#[PathParams]
pub struct HomePathParams {
    pub home_id: u32,
}

pub fn get_home(PathParams(HomePathParams { home_id }): PathParams<HomePathParams>) -> Response {
    Response::ok().set_typed_body(format!("{}", home_id))
}

#[PathParams]
pub struct RoomPathParams {
    pub home_id: u32,
    // This is not a valid type for a route parameter
    pub room_id: Vec<u32>,
}

pub fn get_room(params: PathParams<RoomPathParams>) -> Response {
    Response::ok().set_typed_body(format!("{}", params.0.home_id))
}

#[PathParams]
pub struct TownPathParams<'a> {
    pub town: std::borrow::Cow<'a, str>,
}

pub fn get_town(params: PathParams<TownPathParams<'_>>) -> Response {
    Response::ok().set_typed_body(format!("{}", params.0.town))
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(
        f!(pavex::request::path::PathParams::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex::request::path::errors::ExtractPathParamsError::into_response
    ));
    bp.route(GET, "/home/:home_id", f!(crate::get_home));
    bp.route(GET, "/home/:home_id/room/:room_id", f!(crate::get_room));
    bp.route(GET, "/town/*town", f!(crate::get_town));
    bp
}
