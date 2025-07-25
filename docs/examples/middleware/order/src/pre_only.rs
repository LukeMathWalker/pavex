//! px:pre_only
use crate::{GET_INDEX, PRE_1, PRE_2}; // px::skip
use pavex::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();

    bp.pre_process(PRE_1);
    bp.pre_process(PRE_2);
    bp.route(GET_INDEX);
    bp // px::skip
}
