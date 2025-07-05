//! px:registration
use pavex::{Blueprint, blueprint::from};

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![pavex]); // px::hl
    // px::skip:start
    bp.nest(crate::buffered_body::blueprint());
    bp.nest(crate::custom_limit::blueprint());
    bp.nest(crate::granular_limits::blueprint());
    bp.nest(crate::no_limit::blueprint());
    bp
    // px::skip:end
}
