//! px:register_one
use super::LOGIN_ERROR_TO_RESPONSE;
use pavex::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.error_handler(LOGIN_ERROR_TO_RESPONSE); // px::hl px::ann:1
    // px::skip:start
    bp
    // px::skip:end
}
