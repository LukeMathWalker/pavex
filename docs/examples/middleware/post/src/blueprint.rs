//! px:registration
use crate::logger::RESPONSE_LOGGER;
use pavex::{blueprint::from, Blueprint};

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.post_process(RESPONSE_LOGGER); // px::ann:1
                                      // px::skip:start
    bp.post_process(crate::compress::COMPRESS);
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
    // px::skip:end
}
