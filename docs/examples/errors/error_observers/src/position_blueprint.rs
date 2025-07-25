//! px:position_matters
use crate::{
    logger::EMIT_ERROR_LOG,
    routes::{INDEX, LOGIN},
};
use pavex::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(INDEX); // px::ann:1
    bp.error_observer(EMIT_ERROR_LOG);
    bp.route(LOGIN); // px::ann:2
                     // px::skip:start
    bp
    // px::skip:end
}
