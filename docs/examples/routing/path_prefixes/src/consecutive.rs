//! px:consecutive
use pavex::{Blueprint, get, http::StatusCode};

pub fn bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.prefix("/prefix").nest({
        let mut bp = Blueprint::new();
        bp.route(HANDLER); // px::hl
        bp
    });
    bp
}

#[get(path = "//path")] // px::hl
pub fn handler() -> StatusCode {
    StatusCode::OK // px::skip
}
