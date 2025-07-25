//! px:registration
use pavex::{Blueprint, blueprint::from};

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.routes(from![crate]); // px::ann:1
    bp.import(from![pavex, crate]); // px::skip
    bp // px::skip
}
