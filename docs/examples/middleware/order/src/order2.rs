//! px:mw_after_nested
use crate::{GET_INDEX, WRAP_1, WRAP_2}; // px::skip
use pavex::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(WRAP_1);
    bp.nest(nested());
    bp.wrap(WRAP_2); // px::hl
    bp // px::skip
}

pub fn nested() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET_INDEX);
    bp // px::skip
}
