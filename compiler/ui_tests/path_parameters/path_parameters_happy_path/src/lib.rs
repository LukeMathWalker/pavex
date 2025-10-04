use pavex::{blueprint::from, Blueprint};
use pavex::{request::path::PathParams, Response};

#[PathParams]
pub struct HomePathParams {
    pub home_id: u32,
}

#[pavex::get(path = "/home/{home_id}")]
pub fn get_home(PathParams(HomePathParams { home_id }): PathParams<HomePathParams>) -> Response {
    Response::ok().set_typed_body(format!("{}", home_id))
}

#[PathParams]
pub struct RoomPathParams {
    pub home_id: u32,
    // This is not a valid type for a route parameter
    pub room_id: Vec<u32>,
}

#[pavex::get(path = "/home/{home_id}/room/{room_id}")]
pub fn get_room(params: PathParams<RoomPathParams>) -> Response {
    Response::ok().set_typed_body(format!("{}", params.0.home_id))
}

#[PathParams]
pub struct TownPathParams<'a> {
    pub town: std::borrow::Cow<'a, str>,
}

#[pavex::get(path = "/town/{*town}")]
pub fn get_town(params: PathParams<TownPathParams<'_>>) -> Response {
    Response::ok().set_typed_body(format!("{}", params.0.town))
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![pavex]);
    bp.routes(from![crate]);
    bp
}
