use pavex::{blueprint::from, Blueprint};
use pavex::Response;

#[pavex::get(path = "/{how}{many}{params}{can}{i}{chain}")]
pub fn too_many_params() -> Response {
    todo!()
}

#[pavex::get(path = "/{*invalid_catch_all}/hey")]
pub fn invalid_catch_all() -> Response {
    todo!()
}

#[pavex::get(path = "/room/{id}")]
pub fn room_id() -> Response {
    todo!()
}

#[pavex::get(path = "/room/{room_id}")]
pub fn conflicting_room_id() -> Response {
    todo!()
}

#[pavex::get(path = "/room/{}")]
pub fn unnamed() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.routes(from![crate]);
    bp
}
