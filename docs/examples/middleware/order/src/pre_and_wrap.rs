//! px:pre_and_wrap
use crate::{GET_INDEX, PRE_1, PRE_2, PRE_3, WRAP_1, WRAP_2}; // px::skip
use pavex::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();

    bp.pre_process(PRE_1);
    bp.wrap(WRAP_1);
    bp.pre_process(PRE_2);
    bp.wrap(WRAP_2);
    bp.pre_process(PRE_3);
    bp.route(GET_INDEX);
    bp // px::skip
}
