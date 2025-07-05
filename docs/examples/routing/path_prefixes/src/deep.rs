//! px:deep
use pavex::{Blueprint, get, http::StatusCode};

pub fn bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.prefix("/home").nest(home_bp()); // px::hl
    bp // px::skip
}

pub fn home_bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.prefix("/{home_id}").nest(room_bp()); // px::hl
    bp // px::skip
}

pub fn room_bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET_ROOM); // px::hl
    bp // px::skip
}

#[get(path = "/room/{room_id}")]
pub fn get_room() -> StatusCode {
    StatusCode::OK
}
