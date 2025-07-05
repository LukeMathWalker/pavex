//! px:id_registration
use crate::auth_error::AUTH_ERROR_HANDLER;
use pavex::{Blueprint, blueprint::from};

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.error_handler(AUTH_ERROR_HANDLER); // px::hl
    // px::skip:start
    bp.import(from![crate, pavex]);
    bp.routes(from![crate]);
    bp
    // px::skip:end
}
