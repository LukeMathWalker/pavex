//! px:import
use pavex::{Blueprint, blueprint::from};

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]); // px:import:ann:1
    // px::skip:start
    bp.routes(from![crate]);
    bp
    // px::skip:end
}
