//! px:registration
use pavex::{blueprint::from, Blueprint};

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![pavex]); // px::hl
                             // px::skip:start
    bp.routes(from![crate]);
    bp
    // px::skip:end
}
