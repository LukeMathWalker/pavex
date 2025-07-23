// px:fallback:start
use pavex::{Blueprint, fallback, http::StatusCode};

pub fn bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.domain("pavex.dev").nest(website_bp());
    bp.domain("api.pavex.dev").prefix("/v1").nest(api_bp());
    // If no domain matches, return a 403.
    bp.fallback(UNKNOWN_DOMAIN); // px::hl
    bp // px::skip
}

#[fallback] // px::hl
pub fn unknown_domain() -> StatusCode {
    StatusCode::FORBIDDEN
}
// px:fallback:end

fn website_bp() -> Blueprint {
    Blueprint::new()
}

fn api_bp() -> Blueprint {
    Blueprint::new()
}
