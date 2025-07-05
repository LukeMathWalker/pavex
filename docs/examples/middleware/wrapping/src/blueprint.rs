//! px:registration
use crate::timeout::TIMEOUT;
use pavex::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(TIMEOUT); // px::ann:1
    // px::skip:start
    bp.wrap(crate::logger::LOGGER);
    bp.wrap(crate::debug::DEBUG_WRAPPER);
    bp.import(pavex::blueprint::from![crate]);
    bp.routes(pavex::blueprint::from![crate]);
    bp
    // px::skip:end
}
