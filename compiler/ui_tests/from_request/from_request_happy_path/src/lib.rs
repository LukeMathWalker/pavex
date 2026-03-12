use pavex::{blueprint::from, Blueprint, FromRequest, Response};

#[derive(FromRequest)]
pub struct GetHomeInput {
    #[path_param]
    pub home_id: u32,
    #[query_param]
    pub extended: Option<bool>,
}

#[pavex::get(path = "/home/{home_id}")]
pub fn get_home(input: GetHomeInput) -> Response {
    Response::ok().set_typed_body(format!("{}", input.home_id))
}

#[derive(FromRequest)]
pub struct GetRoomInput {
    #[path_param]
    pub home_id: u32,
    #[path_param]
    pub room_id: u32,
}

#[pavex::get(path = "/home/{home_id}/room/{room_id}")]
pub fn get_room(input: GetRoomInput) -> Response {
    Response::ok().set_typed_body(format!("{}", input.home_id))
}

#[derive(FromRequest)]
pub struct GetTownInput {
    #[path_param]
    pub town: String,
}

#[pavex::get(path = "/town/{*town}")]
pub fn get_town(input: GetTownInput) -> Response {
    Response::ok().set_typed_body(format!("{}", input.town))
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate, pavex]);
    bp.routes(from![crate]);
    bp
}
