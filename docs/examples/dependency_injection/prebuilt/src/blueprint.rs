//! px:registration
use pavex::{Blueprint, blueprint::from};

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]); // px::ann:1
    bp.routes(from![crate]); // px::skip
    bp // px::skip
}
