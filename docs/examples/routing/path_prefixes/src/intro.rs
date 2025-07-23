//! px:intro
use pavex::{Blueprint, blueprint::from, get, http::StatusCode};

pub fn bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.prefix("/home").routes(from![self::home]); // px::hl
    bp.route(INDEX); // px::hl
    bp // px::skip
}

#[get(path = "/")]
pub fn index() -> StatusCode {
    StatusCode::OK // px::skip
}

pub mod home {
    use super::*;

    #[get(path = "/")]
    pub fn list_homes() -> StatusCode {
        StatusCode::OK // px::skip
    }

    #[get(path = "/{home_id}")]
    pub fn get_home() -> StatusCode {
        StatusCode::OK // px::skip
    }
}
