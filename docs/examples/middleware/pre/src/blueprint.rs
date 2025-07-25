//! px:registration
use crate::redirect::REDIRECT_TO_NORMALIZED;
use pavex::{Blueprint, blueprint::from};

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.pre_process(REDIRECT_TO_NORMALIZED); // px::ann:1
    // px::skip:start
    bp.pre_process(crate::reject_anonymous::REJECT_ANONYMOUS);
    bp.routes(from![crate]);
    bp.import(from![crate]);
    bp
    // px::skip:end
}
