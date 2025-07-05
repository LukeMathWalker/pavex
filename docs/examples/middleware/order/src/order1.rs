//! px:mw_after_handler
use crate::{GET_INDEX, WRAP_1, WRAP_2}; // px::skip
use pavex::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(WRAP_1);
    bp.route(GET_INDEX);
    bp.wrap(WRAP_2); // px::hl
    bp // px::skip
}
