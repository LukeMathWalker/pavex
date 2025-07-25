//! px:registration
use pavex::{Blueprint, blueprint::from};

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![pavex]); // px::hl
    // px::skip:start
    bp.routes(from![crate]);
    bp
    // px::skip:end
}
