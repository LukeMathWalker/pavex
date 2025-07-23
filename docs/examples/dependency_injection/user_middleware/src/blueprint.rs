//! px:registration
use pavex::{Blueprint, blueprint::from};

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]); // px::ann:1
    // px::skip:start
    bp.pre_process(crate::authentication::REJECT_ANONYMOUS);
    bp.route(crate::routes::GREET);
    bp
    // px::skip:end
}
